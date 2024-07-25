use std::collections::HashMap;
use async_trait::async_trait;
use leptos::*;
use immt_core::building::buildstate::{QueueEntry, QueueMessage};
use immt_core::building::formats::{BuildJobSpec, FormatOrTarget, SourceFormatId};
use immt_core::uris::archives::ArchiveId;
use immt_core::utils::time::Delta;
use immt_core::utils::VecMap;
use crate::accounts::{if_logged_in, login_status, LoginState};
use crate::components::logging::Log;
use crate::console_log;
use crate::utils::errors::IMMTError;
use crate::utils::WebSocket;

#[server(
    prefix="/api/buildqueue",
    endpoint="enqueue",
    input=server_fn::codec::GetUrl,
    output=server_fn::codec::Json
)]
pub async fn enqueue(archive:Option<String>,group:Option<String>,target:SourceFormatId,path:Option<String>,all:bool) -> Result<(),ServerFnError<IMMTError>> {
    use immt_controller::{controller,ControllerTrait};
    match login_status().await? {
        LoginState::Admin => {
            //println!("building [{group:?}][{archive:?}]/{path:?}: {target} ({all})");
            let spec = match (archive,group,path) {
                (None,Some(_),Some(_)) | (None,None,_) => {
                    return Err(ServerFnError::MissingArg("Must specify either an archive with optional path or a group".into()))
                },
                (Some(a),_,Some(p)) => {
                    let id : ArchiveId = a.parse().map_err(|_| IMMTError::InvalidArgument("archive".to_string()))?;
                    BuildJobSpec::Path {id,rel_path:p.into(),target:FormatOrTarget::Format(target),stale_only:!all}
                },
                (Some(a),_,_) => {
                    let id : ArchiveId = a.parse().map_err(|_| IMMTError::InvalidArgument("archive".to_string()))?;
                    BuildJobSpec::Archive {id,target:FormatOrTarget::Format(target),stale_only:!all}
                },
                (_,Some(a),_) => {
                    let id : ArchiveId = a.parse().map_err(|_| IMMTError::InvalidArgument("group".to_string()))?;
                    BuildJobSpec::Group {id,target:FormatOrTarget::Format(target),stale_only:!all}
                }
            };
            let controller = controller();
            {controller.build_queue().enqueue(spec,controller)}
            Ok(())
        },
        _ => Err(IMMTError::AccessForbidden.into())
    }
}

#[server(
    prefix="/api/buildqueue",
    endpoint="get_queues",
    input=server_fn::codec::GetUrl,
    output=server_fn::codec::Json
)]
pub async fn get_queues() -> Result<Vec<String>,ServerFnError<IMMTError>> {
    use immt_controller::{controller,ControllerTrait};
    match login_status().await? {
        LoginState::Admin => {
            Ok(controller().build_queue().queues()
                .iter().map(|q| q.id().to_string()).collect())
        },
        _ => Err(IMMTError::AccessForbidden.into())
    }
}

#[server(
    prefix="/api/buildqueue",
    endpoint="run",
    input=server_fn::codec::GetUrl,
    output=server_fn::codec::Json
)]
pub async fn run(id:String) -> Result<(),ServerFnError<IMMTError>> {
    use immt_controller::{controller,ControllerTrait};
    //println!("Running queue {id}");
    match login_status().await? {
        LoginState::Admin => {
            let ctrl = controller();
            ctrl.build_queue().start(&id,ctrl.clone());
            Ok(())
        },
        _ => Err(IMMTError::AccessForbidden.into())
    }
}

pub(crate) struct QueueSocket {
    #[cfg(feature="client")]
    socket: web_sys::WebSocket,
    #[cfg(feature="server")]
    listener: immt_api::utils::asyncs::ChangeListener<QueueMessage>
}
#[async_trait]
impl WebSocket<(),QueueMessage> for QueueSocket {
    const SERVER_ENDPOINT: &'static str = "/dashboard/queue/ws";
    #[cfg(feature="server")]
    async fn new(account:LoginState,db:sea_orm::DatabaseConnection) -> Option<Self> {
        use immt_api::controller::Controller;
        if account == LoginState::Admin {
            let listener = immt_controller::controller().build_queue().listener();
            Some(Self {listener})
        } else {None}
    }
    #[cfg(feature="client")]
    fn new(ws: leptos::web_sys::WebSocket) -> Self { Self{socket:ws} }
    #[cfg(feature="server")]
    async fn next(&mut self) -> Option<QueueMessage> {
        self.listener.read().await
    }
    #[cfg(feature="server")]
    async fn handle_message(&mut self,msg:()) -> Option<QueueMessage> {None}
    #[cfg(feature="server")]
    async fn on_start(&mut self,socket:&mut axum::extract::ws::WebSocket) {
        use immt_controller::{controller,ControllerTrait};
        let msgs = controller().build_queue().queues().iter().flat_map(|q| {
            if q.running() {
                serde_json::to_string(&q.state()).ok()
            } else {
                let entries = q.get_list();
                serde_json::to_string(&QueueMessage::Idle {id:q.id().to_string(),entries}).ok()
            }
        }).collect::<Vec<_>>();
        //println!("HERE: {msg}");
        for msg in msgs {
            match socket.send(axum::extract::ws::Message::Text(msg)).await {
                Err(e) => tracing::info!("Error sending message: {e}"),
                _ => ()
            }
        }
        while let Some(msg) = self.listener.get() {
            match socket.send(axum::extract::ws::Message::Text(serde_json::to_string(&msg).unwrap())).await {
                Err(e) => tracing::info!("Error sending message: {e}"),
                _ => ()
            }
        }
    }
    #[cfg(feature="client")]
    fn socket(&mut self) -> &mut leptos::web_sys::WebSocket {&mut self.socket }
}

#[component]
pub fn QueuesTop() -> impl IntoView {
    view!{<Await future = || get_queues() let:queues blocking=true>{
        match queues {
            Ok(q) if q.is_empty() => view!(<div>"No running queues"</div>).into_view(),
            Ok(q) => view!(<QueueTabs queues=q.clone()/>).into_view(),
            _ => view!(<div>"Error"</div>).into_view()
        }
    }</Await> }
}
/*
#[derive(Clone)]
struct QueueDiv {
    div_ref:NodeRef<html::Div>,
    entries:VecMap<String,NodeRef<html::Div>>
}*/

#[derive(Clone,serde::Serialize,serde::Deserialize,PartialEq)]
struct Entry(QueueEntry);
impl AsRef<QueueEntry> for Entry {
    fn as_ref(&self) -> &QueueEntry { &self.0 }

}
impl Entry {
    fn as_view(&self) -> impl IntoView {
        view!(
            <li><b>{self.0.target.to_string()}</b>{format!(" [{}]/{} ({}/{})",self.0.archive,self.0.rel_path,self.0.step.0+1,self.0.step.1)}</li>
        )
    }
    fn id(&self) -> String { self.0.id() }
}

#[derive(Clone)]
struct AllQueues {
    selected:RwSignal<String>,
    queues:RwSignal<VecMap<String,RwSignal<QueueData>>>
}
impl AllQueues {
    fn new(ids:Vec<String>) -> Self {
        let selected = ids.first().cloned().unwrap_or("".to_string());
        let queues = ids.into_iter().map(|id| (id,create_rw_signal(QueueData::Empty))).collect();
        Self {selected:create_rw_signal(selected.clone()),queues:RwSignal::new(queues)}
    }
}

#[derive(Clone)]
enum QueueData {
    Idle(RwSignal<Vec<Entry>>),
    Running(RunningQueue),
    Empty
}
impl QueueData {
}

#[derive(Clone,serde::Serialize,serde::Deserialize)]
struct RunningQueue {
    running:RwSignal<Vec<Entry>>,
    queue:RwSignal<Vec<Entry>>,
    blocked:RwSignal<Vec<Entry>>,
    failed:RwSignal<Vec<Entry>>,
    finished:RwSignal<Vec<Entry>>,
    eta:RwSignal<Delta>
}

#[island]
fn QueueTabs(queues:Vec<String>) -> impl IntoView {
    use thaw::*;
    use wasm_bindgen::JsCast;
    let queues = AllQueues::new(queues);
    let qc = queues.clone();
    let show = create_rw_signal(false);
    let div_ref = create_node_ref::<html::Div>();
    let refcl= div_ref.clone();

    QueueSocket::force_start(move |msg| {
        let queues = &qc;
        match msg {
            QueueMessage::Idle {id,entries} => {
                let idle = QueueData::Idle(RwSignal::new(entries.into_iter().map(|e| Entry(e)).collect()));
                queues.queues.update(|v| {
                    match v.get_mut(&id) {
                        Some(s) => s.set(idle),
                        None => v.insert(id.clone(),create_rw_signal(idle))
                    }
                })
            }
            QueueMessage::Started {id, queue,blocked,failed,done,eta } => {
                queues.queues.update(|v| {
                    let q = QueueData::Running(RunningQueue {
                        running:RwSignal::new(Vec::new()),
                        queue:RwSignal::new(queue.into_iter().map(|e| Entry(e)).collect()),
                        blocked:RwSignal::new(blocked.into_iter().map(|e| Entry(e)).collect()),
                        failed:RwSignal::new(failed.into_iter().map(|e| Entry(e)).collect()),
                        finished:RwSignal::new(done.into_iter().map(|e| Entry(e)).collect()),
                        eta:RwSignal::new(eta)
                    });
                    match v.get_mut(&id) {
                        Some(s) => s.set(q),
                        None => v.insert(id.clone(),create_rw_signal(q))
                    }
                })
            }
            QueueMessage::TaskStarted{ id, entry,eta} => {
                if let Some(q) = queues.queues.get_untracked().get(&id) {
                    if let QueueData::Running(r) = q.get_untracked() {
                        if let Some((i,_)) = r.queue.get_untracked().iter().enumerate()
                            .find(|(i,e)| e.0 == entry) {
                            r.queue.update(|v| {v.remove(i);})
                        } else if let Some((i,_)) = r.blocked.get_untracked().iter().enumerate()
                            .find(|(i,e)| e.0 == entry) {
                            r.blocked.update(|v| {v.remove(i);})
                        };
                        r.running.update(|v| v.push(Entry(entry)));
                        r.eta.update(|e| e.update_average(0.95,eta));
                    }
                }
            }
            QueueMessage::TaskDoneRequeued {id, entry,index,eta} => {
                if let Some(q) = queues.queues.get_untracked().get(&id) {
                    if let QueueData::Running(r) = q.get_untracked() {
                        if let Some((i,_)) = r.running.get_untracked().iter().enumerate()
                            .find(|(i,e)| e.0 == entry) {
                            r.running.update(|v| {v.remove(i);})
                        }
                        r.queue.update(|v| v.insert(index,Entry(entry)));
                        r.eta.update(|e| e.update_average(0.95,eta));
                    }
                }
            }
            QueueMessage::TaskDoneFinished {id, entry,eta} => {
                if let Some(q) = queues.queues.get_untracked().get(&id) {
                    if let QueueData::Running(r) = q.get_untracked() {
                        if let Some((i, _)) = r.running.get_untracked().iter().enumerate()
                            .find(|(i, e)| e.0 == entry) {
                            r.running.update(|v| { v.remove(i); })
                        }
                        r.finished.update(|v| v.push(Entry(entry)));
                        r.eta.update(|e| e.update_average(0.95,eta));
                    }
                }
            }
            QueueMessage::TaskFailed {id, entry,eta} => {
                if let Some(q) = queues.queues.get_untracked().get(&id) {
                    if let QueueData::Running(r) = q.get_untracked() {
                        if let Some((i, _)) = r.running.get_untracked().iter().enumerate()
                            .find(|(i, e)| e.0 == entry) {
                            r.running.update(|v| { v.remove(i); })
                        }
                        r.failed.update(|v| v.push(Entry(entry)));
                        r.eta.update(|e| e.update_average(0.95,eta));
                    }
                }
            }
        }
        if !show.get_untracked() {
            show.set(true);
            let q = queues.clone();
            refcl.get_untracked().unwrap().replace_children_with_node_1(
                view!(<div><Tabs value=q.selected>
                    <For each=move || q.queues.clone().get() key=|e| e.0.clone() children=move |e| view!(
                        <Tab key=&e.0 label=e.0.clone()>{QueueTop(e.0,e.1)}</Tab>
                    )/>
                </Tabs></div>).dyn_ref().unwrap()
            )
        }
        None
    });
    view!(<div node_ref=div_ref />)
}

fn QueueTop(id:String,ls:RwSignal<QueueData>) -> impl IntoView {
    move || match ls.get() {
        QueueData::Idle(v) => {
            IdleQueue(id.clone(),v).into_view()
        },
        QueueData::Running(r) => {
            RunningQueue(id.clone(),r).into_view()
        }
        _ => view!(<div>"Other"</div>).into_view()
    }
}

// #[island]
fn IdleQueue(id:String, ls:RwSignal<Vec<Entry>>) -> impl IntoView {
    use thaw::*;
    let act = create_action(move |_| run(id.clone()));
    view!{
        <Space justify=SpaceJustify::End>{move ||
            view!(<Button on_click=move |_| act.dispatch(())>"Run"</Button>)
        }</Space>
        <ul>
          <For each=move || ls.get() key=|e| e.id() children=|e| e.as_view()/>
        </ul>
    }
}

pub fn RunningQueue(id:String, queue:RunningQueue) -> impl IntoView {
    use thaw::*;
    use leptos_meta::Stylesheet;
    let RunningQueue {running,queue,blocked,failed, finished,eta } = queue;
    move || view!{<Space>
        <Space align=SpaceAlign::Start><Layout>
            <div>"ETA: "{move || eta.get().to_string()}</div>
            <h3 id="running">Running</h3>
            <ul><For each=move || running.get() key=|e| e.id() children=|e| e.as_view()/></ul>
            <h3 id="queued">Queued</h3>
            <ul><For each=move || queue.get() key=|e| e.id() children=|e| e.as_view()/></ul>
            <h3 id="blocked">Blocked</h3>
            <ul><For each=move || blocked.get() key=|e| e.id() children=|e| e.as_view()/></ul>
            <h3 id="failed">Failed</h3>
            <ul><For each=move || failed.get() key=|e| e.id() children=|e| e.as_view()/></ul>
            <h3 id="finished">Finished</h3>
            <ul><For each=move || finished.get() key=|e| e.id() children=|e| e.as_view()/></ul>
        </Layout></Space>
        <Space align=SpaceAlign::End><div style="position:absolute;"><Anchor>
            <AnchorLink title="Running" href="#running"/>
            <AnchorLink title="Queued" href="#queued"/>
            <AnchorLink title="Blocked" href="#blocked"/>
            <AnchorLink title="Failed" href="#failed"/>
            <AnchorLink title="Finished" href="#finished"/>
        </Anchor></div></Space>
    </Space>}
}