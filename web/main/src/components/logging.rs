//stylance::import_crate_style!(loglist,"style/components/loglist.scss");
#[allow(non_upper_case_globals)]
mod css {
    pub const immt_log_frame: &str = "immt-log-frame";
    pub const immt_warn_frame: &str = "immt-warn-frame";
    pub const immt_log_list: &str = "immt-log-list";
    pub const immt_log_elem: &str = "immt-log-elem";
    pub const immt_spinner_inline: &str = "immt-spinner-inline";
    pub const immt_log_error: &str = "immt-log-error";
    pub const immt_log_warn: &str = "immt-log-warn";
    pub const immt_log_info: &str = "immt-log-info";
    pub const immt_log_debug: &str = "immt-log-debug";
    pub const immt_log_trace: &str = "immt-log-trace";
}
use css::*;

use crate::{console_log, socket};
use std::time::Duration;
use leptos::*;
use immt_core::utils::logs::{LogFileLine, LogLevel, LogMessage, LogSpan, LogTree, LogTreeElem};
use immt_core::utils::time::Timestamp;
use immt_core::utils::VecMap;
use crate::accounts::LoginState;

#[server(
    prefix="/api/log",
    endpoint="full",
    input=server_fn::codec::GetUrl,
    output=server_fn::codec::Json
)]
pub async fn full_log() -> Result<LogTree,ServerFnError<String>> {
    full_log_i().await
        .map_err(|_| ServerFnError::ServerError("".to_string()))
}
#[cfg(feature="server")]
//#[tracing::instrument(level="debug",skip_all,target="server","loading log file")]
async fn full_log_i() -> Result<LogTree,()> {
    use immt_controller::{Controller,ControllerTrait,controller};
    use tokio::io::AsyncBufReadExt;

    // tokio::time::sleep(Duration::from_secs_f32(5.0)).await;

    let path = controller().log_file();

    let reader = tokio::io::BufReader::new(tokio::fs::File::open(path).await.map_err(|e| ())?);
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
type Frames = VecMap<String,(NodeRef<html::Ul>,NodeRef<html::Span>,Timestamp)>;

socket!(LogViewer<(),Log> @ "/log/ws" => {
    last_ping: std::time::Instant,
    listener: immt_api::utils::asyncs::ChangeListener<LogFileLine<String>>,
    #[cfg(feature="accounts")]
    identity:Option<LoginState>
}{
    fn new(now: std::time::Instant,r:&actix_web::HttpRequest) -> Self {
        #[cfg(feature="accounts")]
        use crate::accounts::ToID;
        use immt_controller::{Controller,ControllerTrait,controller};
        let listener = controller().log_listener();
        Self {last_ping: now, listener,
            #[cfg(feature="accounts")]
            identity: actix_identity::IdentityExt::get_identity(r).ok().map(|i| i.into_login_state()).flatten()
        }
    }
    fn last_ping(&mut self) -> &mut std::time::Instant {
        &mut self.last_ping
    }
    fn every(&mut self,_:bool, _: &mut Self::Context) -> Option<Log> {
        if let Some(n) = self.listener.get() {
            Some(Log::Update(n))
        } else {None}
    }
    fn on_start(&mut self, ctx: &mut Self::Context) {
        use actix::prelude::*;
        #[cfg(feature="accounts")]
        {
            if self.identity != Some(LoginState::Admin) {
                actix_web_actors::ws::WebsocketContext::stop(ctx);
                return
            }
        }
        use actix::{ContextFutureSpawner, WrapFuture,ActorFutureExt};
        let _ = full_log().into_actor(self).map(|r,_,ctx| {
            ctx.text(serde_json::to_string(&Log::Initial(r.unwrap())).unwrap());
        }).spawn(ctx);
    }
});

#[component]
pub fn FullLog() -> impl IntoView { template!{<TopLog/>} }

struct LogState {
    log_frame:NodeRef<html::Ul>,
    warn_frame:NodeRef<html::Ul>,
    spinners: (NodeRef<html::Div>,NodeRef<html::Div>),
    frames:Frames,
}
#[island]
fn TopLog() -> impl IntoView {
    use crate::utils::ws::WS;
    use thaw::Spinner;
    let log_frame = create_node_ref::<html::Ul>();
    let warn_frame = create_node_ref::<html::Ul>();
    let (spinner_a,spinner_b) = (create_node_ref::<html::Div>(),create_node_ref::<html::Div>());
    let mut state = LogState {
        log_frame,
        warn_frame,
        spinners: (spinner_a,spinner_b),
        frames: Frames::default(),
    };
    LogViewer::run(move |l| {
        #[cfg(feature="client")]
        {client::ws(&mut state,l);}
        None
    });
    template!{
        <div class=immt_log_frame><div node_ref=spinner_a><Spinner/></div>
            <ul node_ref=log_frame/>
        </div>
        <div class=immt_warn_frame><div node_ref=spinner_b><Spinner/></div>
            <ul node_ref=warn_frame/>
        </div>
    }
}

#[cfg(feature="client")]
mod client {
    use leptos::*;
    use wasm_bindgen::JsCast;
    use super::*;

    pub(crate) fn ws(state:&mut LogState,l:Log) {
        match l {
            Log::Initial(tree) => populate(state,tree),
            Log::Update(up) => update(state,up)
        }
    }
    fn populate(state:&mut LogState,tree:LogTree) {
        fn do_tree_elems(children:Vec<LogTreeElem>,elem:&HtmlElement<html::Ul>,warn_frame:&HtmlElement<html::Ul>,frames:&mut Frames) {
            for c in children {
                match c {
                    LogTreeElem::Message(LogMessage {message,timestamp,target,level,args}) => {
                        if level >= LogLevel::WARN {
                            warn_frame.append_child(template!(<li><LogLine timestamp message=message.clone() target=target.clone() level args=args.clone() /></li>).dyn_ref().unwrap()).unwrap();
                        }
                        elem.append_child(template!(<li><LogLine timestamp message target level args /></li>).dyn_ref().unwrap()).unwrap();
                    }
                    LogTreeElem::Span(LogSpan {id,name,timestamp,target,level,args,children,closed}) => {
                        let message = if let Some(closed) = closed {
                            format!("{} (finished after {})",name,closed.since(timestamp))
                        } else {name};
                        let nchildren = create_node_ref::<html::Ul>();
                        let span_ref = create_node_ref::<html::Span>();
                        let sr = span_ref.clone();
                        let line = template!{
                                    <li class=immt_log_elem><details>
                                        <summary><LogLine timestamp message target level args spinner=closed.is_none() span_ref=sr/></summary>
                                        <ul node_ref=nchildren/>
                                    </details></li>
                                };
                        elem.append_child(line.dyn_ref().unwrap()).unwrap();
                        if closed.is_none() { frames.insert(id,(nchildren,span_ref,timestamp)) }
                        let nchildren = nchildren.get_untracked().unwrap();
                        do_tree_elems(children,&nchildren,&warn_frame,frames);
                    }
                }
            }
        }
        let Some(log_frame) = state.log_frame.get_untracked() else {return};
        let Some(warn_frame) = state.warn_frame.get_untracked() else {return};
        do_tree_elems(tree.children,&log_frame,&warn_frame,&mut state.frames);

        if let Some(n) = state.spinners.0.get_untracked() { n.remove(); }
        if let Some(n) = state.spinners.1.get_untracked() { n.remove(); }
    }
    fn update(state:&mut LogState,line:LogFileLine<String>) {
        let Some(log_frame) = state.log_frame.get_untracked() else {return};
        let Some(warn_frame) = state.warn_frame.get_untracked() else {return};
        match line {
            LogFileLine::Message {message,timestamp,target,level,args,span} => {
                if level >= LogLevel::WARN {
                    warn_frame.append_child(template!(<li><LogLine timestamp=timestamp.clone() message=message.clone() target=target.clone() level args=args.clone() /></li>).dyn_ref().unwrap()).unwrap();
                }
                let line = template!(<li><LogLine timestamp message target level args /></li>);
                if span.is_none() {
                    log_frame.append_child(line.dyn_ref().unwrap()).unwrap();
                } else if let Some((frame,_,_)) = state.frames.get(&span.unwrap()) {
                    let children = frame.get_untracked().unwrap();
                    children.append_child(line.dyn_ref().unwrap()).unwrap();
                }
            }
            LogFileLine::SpanOpen {id,name,timestamp,target,level,args,parent} => {
                let children = create_node_ref::<html::Ul>();
                let span_ref = create_node_ref::<html::Span>();
                let sr = span_ref.clone();
                let line = template! {
                    <li class=immt_log_elem><details>
                        <summary><LogLine timestamp message=name target level args spinner=true span_ref/></summary>
                        <ul node_ref=children/>
                    </details></li>
                };
                if parent.is_none() {
                    log_frame.append_child(line.dyn_ref().unwrap()).unwrap();
                } else if let Some((frame,_,_)) = state.frames.get(&parent.unwrap()) {
                    let children = frame.get_untracked().unwrap();
                    children.append_child(line.dyn_ref().unwrap()).unwrap();
                }
                state.frames.insert(id, (children,sr,timestamp));
            }
            LogFileLine::SpanClose {id,timestamp,parent} => {
                if let Some((frame,span,started)) = state.frames.remove(&id) {
                    let span = span.get_untracked().unwrap();
                    let message = format!("{} (finished after {})",span.text_content().unwrap(),timestamp.since(started));
                    span.set_text_content(Some(&message));
                }
            }
        }
    }
}

#[component]
fn LogLine(message:String,timestamp:Timestamp,target:Option<String>,level:LogLevel,args:VecMap<String,String>,#[prop(optional)] spinner:bool,#[prop(optional)] span_ref:Option<NodeRef<html::Span>>) -> impl IntoView {
    use thaw::{Spinner, SpinnerSize};
    use std::fmt::Write;
    let cls = class_from_level(level);
    let mut str = format!("{} <{}> ",timestamp,level);
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
    let span_ref = span_ref.unwrap_or(NodeRef::default());
    if spinner {
        template! {<span class=cls node_ref=span_ref>
            <span class=immt_spinner_inline>
            <Spinner size=SpinnerSize::Tiny/>
            </span>{str}
        </span>}
    } else {template! {<span class=cls node_ref=span_ref>{str}</span>}}
}

fn class_from_level(lvl:LogLevel) -> &'static str {
    match lvl {
        LogLevel::ERROR => immt_log_error,
        LogLevel::WARN => immt_log_warn,
        LogLevel::INFO => immt_log_info,
        LogLevel::DEBUG => immt_log_debug,
        LogLevel::TRACE => immt_log_trace,
    }
}