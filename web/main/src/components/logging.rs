use async_trait::async_trait;
use leptos::{prelude::*,html};
use immt_core::utils::logs::{LogFileLine, LogLevel, LogMessage, LogSpan, LogTree, LogTreeElem};
use immt_utils::{time::Timestamp,prelude::*};
use leptos::html::HtmlElement;
use crate::css;
use crate::utils::WebSocket;

#[cfg(feature="server")]
//#[tracing::instrument(level="debug",skip_all,target="server","loading log file")]
async fn full_log_i() -> Result<LogTree,()> {
    use immt_controller::{Controller,controller};
    use tokio::io::AsyncBufReadExt;

    // tokio::time::sleep(Duration::from_secs_f32(5.0)).await;

    let path = controller().log_file();

    let reader = tokio::io::BufReader::new(tokio::fs::File::open(path).await.map_err(|_| ())?);
    let mut lines = reader.lines();
    let mut parsed = Vec::new();
    while let Ok(Some(line)) = lines.next_line().await {
        if !line.is_empty() {
            if let Some(line) = LogFileLine::parse(&line) {
                parsed.push(line.to_owned());
            }
        }
    }
    let tree : LogTree = parsed.into();
    Ok(tree)
}


#[derive(Debug,Clone,serde::Serialize,serde::Deserialize)]
pub enum Log {
    Initial(LogTree),
    Update(LogFileLine<String>),
}

#[derive(Debug,Copy,Clone,serde::Serialize,serde::Deserialize)]
struct LogSignals {
    top:RwSignal<Vec<LogEntrySignal>>,
    open_span_paths:RwSignal<VecMap<String,Vec<usize>>>,
    warnings:RwSignal<Vec<(String,LogMessage)>>
}
#[derive(Debug,Clone,serde::Serialize,serde::Deserialize)]
enum LogEntrySignal {
    Simple(String,LogMessage),
    Span(String,SpanSignal)
}
impl LogEntrySignal {
    fn id(&self) -> &str {
        match self {
            LogEntrySignal::Simple(id,_) => id,
            LogEntrySignal::Span(id,_) => id
        }
    }
}

#[derive(Debug,Clone,serde::Serialize,serde::Deserialize)]
enum SpanMessage {
    Open{ name:String, timestamp:Timestamp},
    Closed(String)
}
#[derive(Debug,Clone,serde::Serialize,serde::Deserialize)]
struct SpanSignal {
    pub message:RwSignal<SpanMessage>,
    pub target:Option<String>,
    pub level:LogLevel,
    pub args:VecMap<String, String>,
    pub children:RwSignal<Vec<LogEntrySignal>>
}

pub(crate) struct LogSocket {
    #[cfg(feature="server")]
    listener: immt_api::utils::asyncs::ChangeListener<LogFileLine<String>>,
    #[cfg(feature="client")]
    socket: leptos::web_sys::WebSocket
}
#[async_trait]
impl WebSocket<(),Log> for LogSocket {
    const SERVER_ENDPOINT: &'static str = "/dashboard/log/ws";
    #[cfg(feature="server")]
    async fn new(account:crate::accounts::LoginState,_db:sea_orm::DatabaseConnection) -> Option<Self> {
        if account == crate::accounts::LoginState::Admin {
            let listener = immt_controller::controller().log_listener();
            Some(Self {listener})
        } else {None}
    }
    #[cfg(feature="client")]
    fn new(ws: leptos::web_sys::WebSocket) -> Self { Self{socket:ws} }
    #[cfg(feature="server")]
    async fn next(&mut self) -> Option<Log> {
        self.listener.read().await.and_then(|m| Some(Log::Update(m)))
    }
    #[cfg(feature="server")]
    async fn handle_message(&mut self,_msg:()) -> Option<Log> {None}
    #[cfg(feature="server")]
    async fn on_start(&mut self,socket:&mut axum::extract::ws::WebSocket) {
        if let Ok(init) = full_log_i().await {
            let _ = socket.send(axum::extract::ws::Message::Text(
                serde_json::to_string(&Log::Initial(init)).unwrap()
            )).await;
        }
    }
    #[cfg(feature="client")]
    fn socket(&mut self) -> &mut leptos::web_sys::WebSocket {&mut self.socket }
}

#[component]
pub fn FullLog() -> impl IntoView {
    css!(logging in "logs.css");
    crate::accounts::if_logged_in(
        || view!(<TopLog/>).into_any(),
        || view!(<div>"You must be logged in to see the logs"</div>).into_any()
    )

}

#[island]
fn TopLog() -> impl IntoView {
    use thaw::*;
    use crate::components::{Tree,Leaf};
    let signals = LogSignals {
        top: RwSignal::new(Vec::new()),
        open_span_paths: RwSignal::new(VecMap::new()),
        warnings: RwSignal::new(Vec::new())
    };
    Effect::new(move |_| {
        #[cfg(feature="client")]
        {
            let _ = LogSocket::start(move |msg| {
                client::ws(signals, msg);
                None
            });
        }
    });
    view!{
        <div class="immt-log-frame">{ move || {
            if signals.top.with(|v| v.is_empty()) {
                view!(<div class="immt-spinner-frame"><Spinner/></div>).into_any()
            } else {view!{<Tree>
                {do_ls(signals.top)}
            </Tree>}.into_any()}
        }}</div>
        <div class="immt-warn-frame">
        <Caption1Strong><span style="color:var(--colorPaletteRedForeground1)">"Warnings"</span></Caption1Strong>{ move || {
            if signals.top.get().is_empty() {
                view!(<div class="immt-spinner-frame"><Spinner/></div>).into_any()
            } else {view!{<Tree>
                <For each=move || signals.warnings.get() key=|e| e.0.clone() children=move |e| view!(
                    <Leaf><LogLine e=e.1/></Leaf>
                )/>
            </Tree>}.into_any()}
        }}</div>
    }
}

#[cfg(feature="client")]
mod client {
    use leptos::*;
    use wasm_bindgen::JsCast;
    use super::*;
    pub(crate) fn ws(signals:LogSignals,l:Log) {
        match l {
            Log::Initial(tree) => populate(signals,tree),
            Log::Update(up) => update(signals,up)
        }
    }
    fn populate(signals:LogSignals,tree:LogTree) {
        signals.open_span_paths.update_untracked(|v| *v = tree.open_span_paths);
        fn add(signal:&mut Vec<LogEntrySignal>,warnings:RwSignal<Vec<(String,LogMessage)>>,e:LogTreeElem) {
            let id = e.id();
            match e {
                LogTreeElem::Message(LogMessage {message,timestamp,target,level,args}) => {
                    if level >= LogLevel::WARN {
                        warnings.update(|v| v.push(
                            (id.clone(),LogMessage {message:message.clone(),timestamp:timestamp.clone(),target:target.clone(),level,args:args.clone()})
                        ));
                    }
                    signal.push(LogEntrySignal::Simple(id,LogMessage {message,timestamp,target,level,args}));
                }
                LogTreeElem::Span(LogSpan {name,timestamp,target,level,args,children,closed}) => {
                    let message = RwSignal::new(if let Some(closed) = closed {
                        SpanMessage::Closed(format!("{} (finished after {})",name,closed.since(timestamp)))
                    } else {SpanMessage::Open{name,timestamp}});
                    let mut nchildren = Vec::new();
                    for c in children {
                        add(&mut nchildren,warnings,c);
                    }
                    let e = SpanSignal {
                        message,
                        target,level,args,children:RwSignal::new(nchildren)
                    };
                    signal.push(LogEntrySignal::Span(id,e));
                }
            }
        }
        signals.top.update(|v|
            for e in tree.children {
                add(v,signals.warnings,e)
            }
        )
    }
    fn update(signals:LogSignals,update:LogFileLine<String>) {
        match update {
            LogFileLine::Message {message,timestamp,target,level,args,span} => {
                let id = LogFileLine::id_from(&message,&args);
                if level >= LogLevel::WARN {
                    signals.warnings.update(|v| v.push((id.clone(), LogMessage { message: message.clone(), timestamp: timestamp.clone(), target: target.clone(), level: level, args: args.clone() })));
                }
                signals.open_span_paths.update_untracked(move |spans| {
                    let mut curr = signals.top;
                    match span.and_then(|id| spans.get(&id)) {
                        Some(v) => {
                            for i in v.iter() {
                                match curr.get_untracked().get(*i) {
                                    Some(LogEntrySignal::Span(_,s)) => curr = s.children,
                                    _ => break
                                }
                            }
                        },
                        None => (),
                    }
                    curr.update(|v| v.push(LogEntrySignal::Simple(id, LogMessage { message, timestamp, target, level, args })));
                })
            }
            LogFileLine::SpanOpen {name,timestamp,target,level,args,parent} => {
                signals.open_span_paths.update_untracked(move |spans| {
                    let id = LogFileLine::id_from(&name,&args);
                    let mut curr = signals.top;
                    let mut nums = Vec::new();
                    match parent.and_then(|id| spans.get(&id)) {
                        Some(v) => {
                            nums = v.clone();
                            for i in v.iter() {
                                match curr.get_untracked().get(*i) {
                                    Some(LogEntrySignal::Span(_, s)) => curr = s.children,
                                    _ => break
                                }
                            }
                        },
                        None => (),
                    }
                    curr.update(|parent| {
                        nums.push(parent.len());
                        parent.push(LogEntrySignal::Span(id, SpanSignal {
                            message: RwSignal::new(SpanMessage::Open { name, timestamp }),
                            target, level, args, children: RwSignal::new(Vec::new())
                        }));
                    })
                });
            }
            LogFileLine::SpanClose {id,timestamp,..} => {
                signals.open_span_paths.update_untracked(move |spans| {
                    if let Some(path) = spans.remove(&id) {
                        fn get(mut iter:std::vec::IntoIter<usize>,ret:&mut Option<(String,RwSignal<SpanMessage>)>,curr:RwSignal<Vec<LogEntrySignal>>) {
                            if let Some(i) = iter.next() {
                                match curr.get_untracked().get(i) {
                                    Some(LogEntrySignal::Span(id,s)) => {
                                        *ret = Some((id.clone(),s.message));
                                        get(iter,ret,s.children)
                                    },
                                    _ => *ret = None
                                }
                            }
                        }
                        let mut ret = None;
                        get(path.into_iter(),&mut ret,signals.top);
                        if let Some((oid,message)) = ret {
                            if oid == id {
                                if let SpanMessage::Open { name, timestamp: old } = message.get_untracked() {
                                    message.set(SpanMessage::Closed(format!("{} (finished after {})", name, timestamp.since(old))));
                                }
                            }
                        }
                    }
                });
            }
        }
    }
}

fn do_ls(v:RwSignal<Vec<LogEntrySignal>>) -> impl IntoView {
    use crate::components::Leaf;
    view!{
        <For each=move || v.get() key=|e| e.id().to_string() children=|e| {
            match e {
                LogEntrySignal::Simple(_,e) => view!(<Leaf><span class="immt-log-elem"><LogLine e/></span></Leaf>).into_any(),
                LogEntrySignal::Span(_,e) => do_span(e).into_any()
            }
        }/>
    }
}
fn do_span(s:SpanSignal) -> impl IntoView {
    use crate::components::{Subtree,WHeader};
    let children = s.children;
    view!{<Subtree>
        <WHeader slot>{move || {let s = s.clone(); match s.message.get() {
            SpanMessage::Open {name,timestamp} => view!(<LogLineHelper message=name timestamp target=s.target level=s.level args=s.args spinner=true/>),
            SpanMessage::Closed(message) => view!(<LogLineHelper message target=s.target level=s.level args=s.args spinner=false />)
        }}}</WHeader>
        {move || do_ls(children)}
    </Subtree>}
}

#[component]
fn LogLine(e:LogMessage) -> impl IntoView {
    let LogMessage {message,timestamp,target,level,args} = e;
    view!(<LogLineHelper message timestamp target level args/>)
}

#[component]
fn LogLineHelper(
    message:String,
    #[prop(optional)] timestamp:Option<Timestamp>,
    target:Option<String>,
    level:LogLevel,
    args:VecMap<String,String>,
    #[prop(optional)] spinner:bool,
) -> impl IntoView {
    use thaw::{Spinner, SpinnerSize};
    use std::fmt::Write;
    let cls = class_from_level(level);
    let mut str = if let Some(timestamp) = timestamp {
        format!("{timestamp} <{level}> ")
    } else {
        format!("<{level}> ")
    };
    if let Some(target) = target {
        write!(str,"[{}] ",target).unwrap();
    }
    str.push_str(&message);
    if !args.is_empty() {
        str.push_str(" (");
        for (k,v) in args {
            write!(str,"{}:{} ",k,v).unwrap();
        }
        str.push(')');
    }
    if spinner {
        view!(<span class=cls>
            <span class="immt-spinner-inline">
            <Spinner size=SpinnerSize::Tiny/>
            </span>{str}
        </span>).into_any()
    } else {view!(<span class=cls>{str}</span>).into_any()}
}

fn class_from_level(lvl:LogLevel) -> &'static str {
    match lvl {
        LogLevel::ERROR => "immt-log-error",
        LogLevel::WARN => "immt-log-warn",
        LogLevel::INFO => "immt-log-info",
        LogLevel::DEBUG => "immt-log-debug",
        LogLevel::TRACE => "immt-log-trace",
    }
}