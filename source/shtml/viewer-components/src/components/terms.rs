use immt_ontology::{content::terms::ArgMode, uris::{ArchiveURITrait, ContentURI, DocumentElementURI, URIWithLanguage, URI}};
use immt_web_utils::{components::{DivOrMrow, Popover, OnClickModal,PopoverSize, PopoverTriggerType}, do_css, inject_css};
use leptos::{context::Provider, either::{Either, EitherOf3}, prelude::*};
use leptos_dyn_dom::OriginalNode;
use shtml_extraction::open::terms::{OpenArg, OpenTerm, PreVar, VarOrSym};

use crate::{components::{IntoLOs, LOs}, SHTMLString};

#[cfg(feature="omdoc")]
enum DomTermArgs {
  Open(Vec<Option<(ArgMode,either::Either<String,Vec<Option<String>>>)>>),
  Closed(Vec<(ArgMode,either::Either<String,Vec<String>>)>)
}

#[derive(Clone)]
pub(super) struct InTermState {
  owner:VarOrSym,
  is_hovered:RwSignal<bool>,
  #[cfg(feature="omdoc")]
  args:RwSignal<DomTermArgs>,
  //replaced:RwSignal<bool>,
  replacable:bool
}

#[derive(Clone)]
struct SkipOne;

impl InTermState {
  fn new(owner:VarOrSym) -> Self {
    Self {
      owner,
      is_hovered:RwSignal::new(false),
      #[cfg(feature="omdoc")]
      args:RwSignal::new(DomTermArgs::Open(Vec::new())),
      //replaced:RwSignal::new(false),
      replacable:false
    }
  }
}


#[cfg(feature="omdoc")]
mod term_replacing {
  use leptos::{either::Either, prelude::*,context::Provider};
  use immt_ontology::{content::terms::ArgMode, narration::notations::{PresentationError, PresenterArgs}, uris::{DocumentElementURI, URI}};
  use leptos_dyn_dom::{DomStringContMath, OriginalNode};
  use shtml_extraction::prelude::SHTMLElements;
  use crate::components::terms::SkipOne;
  use super::super::do_components;
  use super::{DomTermArgs, InTermState};

  pub(crate) const DO_REPLACEMENTS: bool = true;

  #[derive(Copy,Clone)]
  struct ArgPres(RwSignal<DomTermArgs>);
  impl PresenterArgs<String> for ArgPres {
    fn single(&self,idx:u8,mode:ArgMode,out:&mut String) -> Result<(),PresentationError> {
        self.0.with_untracked(|args| {
          let DomTermArgs::Closed(v) = args else {unreachable!()};
          let Some((_,either::Left(s))) = v.get((idx - 1) as usize) else {
            return Err(PresentationError::ArgumentMismatch)
          };
          out.push_str(s);
          Ok(())
        })
    }
    fn sequence(&self,idx:u8,mode:ArgMode)
            -> std::result::Result<impl Iterator<Item = impl FnOnce(&mut String) -> Result<(),PresentationError>>,PresentationError> {
      self.0.with_untracked(|args| {
        let DomTermArgs::Closed(v) = args else {unreachable!()};
        let v = match v.get((idx -1) as usize) {
          None => return Err(PresentationError::ArgumentMismatch),
          Some((_,either::Left(s))) => vec![s.clone()],
          Some((_,either::Right(v))) => v.clone()
        };
        let ret = v.into_iter().map(|s:String| {
          move |out:&mut String| {
            out.push_str(&s);
            Ok(())
          }
        });
        Ok(ret)
      })
    }
  }

  pub(super) fn replacable(
    mut head:InTermState,
    elements:SHTMLElements,
    orig:OriginalNode,
    on_load:RwSignal<bool>,
    is_var:bool,
    uri:URI,
    notation_signal:RwSignal<Option<DocumentElementURI>>
  ) -> impl IntoView {
    let args = head.args;
    
    head.replacable = true;
    
    let _ = Effect::new(move || if on_load.get() {
      if args.with_untracked(|e| matches!(e,DomTermArgs::Open(_))) {
        args.update(|args| {
          let DomTermArgs::Open(v) = args else {unreachable!()};
          //tracing::trace!("Closing term with {} arguments",v.len());
          let mut v = std::mem::take(v).into_iter();
          let mut ret = Vec::new();
          while let Some(Some((mode,s))) = v.next() {
            match (mode,s) {
              (ArgMode::Normal|ArgMode::Binding, either::Left(s)) => ret.push((mode,either::Left(s))),
              (ArgMode::Sequence|ArgMode::BindingSequence, either::Right(v)) => {
                let mut r = Vec::new();
                let mut iter = v.into_iter();
                while let Some(Some(s)) = iter.next() {
                  r.push(s);
                }
                for a in iter {
                  if a.is_some() {
                    tracing::error!("Missing argument in associative argument list");
                  }
                }
                ret.push((mode,either::Right(r)));
              }
              (ArgMode::Sequence|ArgMode::BindingSequence,either::Left(s)) => ret.push((mode,either::Right(vec![s]))),
              (ArgMode::Normal|ArgMode::Binding,_) => tracing::error!("Argument of mode {mode:?} has multiple entries"),
            }
          }
          for e in v {
            if e.is_some() {
              tracing::error!("Missing argument in term");
            }
          }
          //tracing::debug!("Arguments: {ret:#?}");
          *args = DomTermArgs::Closed(ret);
        });
      }
    });

    let substituted = RwSignal::new(false);

    let oclone = orig.deep_clone();
    view!{<Provider value=Some(head)>{move || {
      macro_rules! orig {
        () => {{
          substituted.update_untracked(|v| *v = false);
          let (o,sig) = if on_load.get_untracked() { 
            (oclone.deep_clone(),RwSignal::new(false))
          } else {
            (orig.clone(),on_load)
          };
          Either::Left(
            //view!(<mrow>{
              do_components::<true>(0,elements.clone(),o,sig).into_any()
            //}</mrow>)
          )
        }};
      }
      if let Some(u) = notation_signal.get() && on_load.get() {
        if substituted.get() {
          let after_subst = RwSignal::new(false);
          let _ = Effect::new(move |_| if after_subst.get() {
            substituted.set(false);
          });
          return Either::Left(
            //view!(<mrow>{
              do_components::<true>(0,elements.clone(),oclone.deep_clone(),after_subst).into_any()
            //}</mrow>)
          )
        }

        let Some(is_op) = args.with(|v| {
          let DomTermArgs::Closed(v) = v else {
            return None
          };
          Some(v.is_empty())
        }) else {
          return orig!();
        };
        let termstr = match (is_op,is_var) {
          (true,true) => "OMV",
          (true,_) => "OMID",
          _ => "OMA"
        };
        let Some(notation) = crate::config::server_config.get_notation(&u) else {
          tracing::error!("Notation {u} not found");
          return orig!()
        };
        //tracing::info!("Rerendering replacable term: {}\n using notation {notation:?}",orig.html_string());
        let args = ArgPres(args);
        let mut html = String::new();
        if let Err(e) = notation.apply_cont(&mut html,None,termstr,&uri,false,&args) {
          tracing::error!("Failed to apply notation {u}: {e}");
          orig!()
        } else {
          //tracing::debug!("Applied notation; {elements:?} original:\n{}\nresult:\n{html}",oclone.html_string());
          substituted.update_untracked(|v| *v = true);
          Either::Right(view!{<mrow class="shtml-comp-replaced"><Provider value=Some(SkipOne)>
            //{view!(
              <DomStringContMath html cont=crate::iterate/>
            //)}
          </Provider></mrow>})
        }
      } else {
        orig!()
      }
    }}</Provider>}
  }
}

#[derive(Clone)]
pub(super) struct DisablePopover;

#[cfg(feature="omdoc")]
pub(super) fn math_term(skip:usize,mut elements:shtml_extraction::prelude::SHTMLElements,orig:OriginalNode,on_load:RwSignal<bool>,t:OpenTerm) -> impl IntoView {
  if term_replacing::DO_REPLACEMENTS {
    Either::Left({
      if use_context::<Option<SkipOne>>().flatten().is_some() {
        tracing::debug!("Skipping");
        let value : Option<SkipOne> = None;
        EitherOf3::A(view!(<Provider value>{super::do_components::<true>(skip+1,elements,orig,on_load).into_any()}</Provider>))
      } else {
        let head = InTermState::new(t.take_head());
        let subst = use_context::<DisablePopover>().is_none();
        if subst {
          let uri = match &head.owner {
            VarOrSym::S(s@ContentURI::Symbol(_)) => Some((false,URI::Content(s.clone()))),
            VarOrSym::V(PreVar::Resolved(v)) => Some((true,URI::Narrative(v.clone().into()))),
            _ => None
          };
          let notation_signal = 
            uri.as_ref().map(|(_,uri)| expect_context::<crate::NotationForces>().get(&uri));
          if let Some(notation_signal) = notation_signal {
            let Some((is_var,uri)) = uri else {unreachable!()};
            //tracing::info!("Here: {elements:?}");
            elements.elems.truncate(elements.elems.len() - (skip + 1));
            return Either::Left(EitherOf3::C(term_replacing::replacable(head,elements,orig,on_load,is_var,uri,notation_signal)));
          }
        }
        
        EitherOf3::B(
          view!(<Provider value=Some(head)>{super::do_components::<true>(skip+1,elements,orig,on_load).into_any()}</Provider>)
        )
      }
    })
  } else {
    Either::Right(
    do_term::<_,true>(t,move || 
      super::do_components::<true>(skip+1,elements,orig,on_load).into_any()
    ))
  }
}

pub(super) fn do_term<V:IntoView+'static,const MATH:bool>(t:OpenTerm,children:impl FnOnce() -> V + Send + 'static) -> impl IntoView + 'static {
  let head = InTermState::new(t.take_head());
  view!{
    <Provider value=Some(head)>{
      children()
    }</Provider>
  }
}

pub(super) fn do_comp<V:IntoView+'static,const MATH:bool>(children:impl FnOnce() -> V + Send + 'static) -> impl IntoView {
  use immt_web_utils::components::PopoverTrigger;
  //tracing::info!("comp!");
  let in_term = use_context::<Option<InTermState>>().flatten();
  if let Some(in_term) = in_term {
    let is_hovered = in_term.is_hovered;
    //tracing::debug!("comp of term {:?}",in_term.owner);
    let is_var = matches!(in_term.owner,VarOrSym::V(_));
    let class = Memo::new(move |_| 
      match (is_hovered.get(), is_var/*,in_term.replaced.get_untracked() */) {
        /*(true, true, true) => "shtml-comp-replaced shtml-var-comp shtml-comp-hover".to_string(),
        (true, false, true) => "shtml-comp-replaced shtml-comp shtml-comp-hover".to_string(),
        (false, true, true) => "shtml-comp-replaced shtml-var-comp".to_string(),
        (false, false, true) => "shtml-comp-replaced shtml-comp".to_string(),*/
        (true, true) => "shtml-var-comp shtml-comp-hover".to_string(),
        (true, false) => "shtml-comp shtml-comp-hover".to_string(),
        (false, true) => "shtml-var-comp".to_string(),
        (false, false) => "shtml-comp".to_string(),
      }
    );
    let do_popover = || use_context::<DisablePopover>().is_none();
    let s = in_term.owner;
    let node_type = if MATH { DivOrMrow::Mrow } else { DivOrMrow::Div };
    
    if do_popover() {
      let ocp = expect_context::<crate::OnClickProvider>().get(&s);
      //let s_click = s.clone();
      Either::Left(view!(
        <Popover node_type class 
          size=PopoverSize::Small
          on_click_signal=ocp
          on_open=move || is_hovered.set(true) 
          on_close=move || is_hovered.set(false)
        >
          <PopoverTrigger class slot>{children()}</PopoverTrigger>
          //<OnClickModal slot>{do_onclick(s_click)}</OnClickModal>
          //<div style="max-width:600px;">
            {match s {
              VarOrSym::V(v) => EitherOf3::A(view!{<span>"Variable "{v.name().last_name().to_string()}</span>}),
              VarOrSym::S(ContentURI::Symbol(s)) => EitherOf3::B(crate::config::get!(definition(s.clone()) = (css,s) => {
                for c in css { do_css(c); }
                Some(view!(<div style="color:black;background-color:white;padding:3px;"><SHTMLString html=s/></div>))
              })),
              VarOrSym::S(ContentURI::Module(m)) =>
                EitherOf3::C(view!{<div>"Module" {m.name().last_name().to_string()}</div>}),
          }}//</div>
        </Popover>
      ))
    } else { Either::Right(children()) }
  } else { Either::Right(children()) }
}

pub(crate) fn do_onclick(uri:VarOrSym) -> impl IntoView {
  use thaw::{Combobox,ComboboxOption,ComboboxOptionGroup,Divider};
  #[cfg(feature="omdoc")]
  let uriclone = uri.clone();
  let s = match uri {
    VarOrSym::V(v) => return EitherOf3::A(view!{<span>"Variable "{v.name().last_name().to_string()}</span>}),
    VarOrSym::S(ContentURI::Module(m)) =>
      return EitherOf3::B(view!{<div>"Module" {m.name().last_name().to_string()}</div>}),
    VarOrSym::S(ContentURI::Symbol(s)) => s
  };
  let name = s.name().last_name().to_string();

  EitherOf3::C(crate::config::get!(get_los(s.clone(),false) = v => {
    let LOs {definitions,examples,..} = v.lo_sort();
    let ex_off = definitions.len();
    let selected = RwSignal::new(definitions.first().map(|_| "0".to_string()));
    let definitions = StoredValue::new(definitions);
    let examples = StoredValue::new(examples);
    view!{
      <div style="display:flex;flex-direction:row;">
        <div style="font-weight:bold;">{name.clone()}</div>
        <div style="margin-left:auto;"><Combobox selected_options=selected placeholder="Select Definition or Example">
          <ComboboxOptionGroup label="Definitions">{
              definitions.with_value(|v| v.iter().enumerate().map(|(i,d)| {
                let line = lo_line(d);
                let value = i.to_string();
                view!{
                  <ComboboxOption text="" value>{line}</ComboboxOption>
                }
            }).collect_view())
          }</ComboboxOptionGroup>
          <ComboboxOptionGroup label="Examples">{
            examples.with_value(|v| v.iter().enumerate().map(|(i,d)| {
              let line = lo_line(d);
              let value = (ex_off + i).to_string();
              view!{
                <ComboboxOption text="" value>{line}</ComboboxOption>
              }
            }).collect_view())
          }</ComboboxOptionGroup>
        </Combobox></div>
      </div>
      <div style="margin:5px;"><Divider/></div>
      {move || {
        let uri = selected.with(|s| s.as_ref().map(|s| {
          let i: usize = s.parse().unwrap_or_else(|_| unreachable!());
          if i < ex_off {
            definitions.with_value(|v:&Vec<DocumentElementURI>| v.as_slice()[i].clone())
          } else {
            examples.with_value(|v:&Vec<DocumentElementURI>| v.as_slice()[i - ex_off].clone())
          }
        }));
        uri.map(|uri| {
          crate::config::get!(paragraph(uri.clone()) = (css,html) => {
            for c in css { do_css(c); }
            view!(<div><SHTMLString html=html/></div>)
          })
        })
      }}
      {#[cfg(feature="omdoc")]{
        if term_replacing::DO_REPLACEMENTS {
          let uri = match &uriclone {
            VarOrSym::S(s@ContentURI::Symbol(_)) => Some((false,URI::Content(s.clone()))),
            VarOrSym::V(PreVar::Resolved(v)) => Some((true,URI::Narrative(v.clone().into()))),
            _ => None
          };
          uri.map(|(is_variable,uri)| {let uricl = uri.clone();crate::config::get!(notations(uri.clone()) = v => {
            if v.is_empty() { None } else {Some({
              let uri = uricl.clone();
              let notation_signal = expect_context::<crate::NotationForces>().get(&uri);
              //let selected = RwSignal::new("None".to_string());
              let selected = RwSignal::new(if let Some(v) = notation_signal.get_untracked() {
                v.to_string()
              } else {
                "None".to_string()
              });
              /*move || if let Some(selected) = notation_signal.get() {
                use thaw::Button;
                let notation = crate::config::server_config.get_notation(&selected);
                let html = notation.display_shtml(false,is_variable,&uri).to_string();
                Either::Left(view!(<div style="margin:5px;"><Divider/></div>
                  <div style="width:100%;"><div style="width:min-content;margin-left:auto;">
                    <Button icon=icondata_ai::AiCloseOutlined on_click=move |_| notation_signal.set(None)>
                      <Provider value=DisablePopover>
                          <crate::SHTMLStringMath html/>
                      </Provider>
                    </Button>
                  </div></div>
                ))
              } else {*/
                Effect::new(move || {
                  let Some(v) = selected.try_get() else {return};
                  if v == "None" { notation_signal.maybe_update(|f|
                    if f.is_some() {
                      *f = None; true
                    } else {false}
                  ); }
                  else {
                    let uri = v.parse().expect("This should be impossible");
                    notation_signal.maybe_update(|v| match v {
                      Some(e) if *e == uri => false,
                      _ => {
                        *v = Some(uri); true
                      }
                    })
                  }
                });
                let uri = uri.clone();
                let v = v.clone();
                Either::Right::<(),_>(view!{<div style="margin:5px;"><Divider/></div>
                <div style="width:100%;"><div style="width:min-content;margin-left:auto;">
                  <Combobox selected_options=selected placeholder="Force Notation">
                    <ComboboxOption text="None" value="None">"None"</ComboboxOption>
                    {let uri = uri.clone();
                      v.into_iter().map(|(u,n)| {let html = n.display_shtml(false,is_variable,&uri).to_string();view!{
                        <ComboboxOption text="" value=u.to_string()>{
                          view!(
                            <Provider value=DisablePopover>
                                <crate::SHTMLStringMath html/>
                            </Provider>
                          )
                        }</ComboboxOption>
                      }}).collect_view()
                    }
                  </Combobox>
                </div></div>})
              //}
            })}
          })})
        } else { None }
      }}
  }}))
}

pub(crate) fn lo_line(uri:&DocumentElementURI) -> impl IntoView + 'static {
  let archive = uri.archive_id().to_string();
  let name = uri.name().to_string();
  let lang = uri.language().flag_svg();
  view!(<div><span>"["{archive}"] "{name}" "</span><div style="display:contents;" inner_html=lang/></div>)
}

pub(super) fn do_arg<V:IntoView + 'static>(orig:OriginalNode,arg:OpenArg,cont:impl FnOnce(OriginalNode) -> V + Send + 'static) -> impl IntoView {
  #[cfg(feature="omdoc")]
  {
    use immt_ontology::shtml::SHTMLKey;
    let tm = use_context::<Option<InTermState>>().flatten();
    if let Some(tm) = tm {
      if tm.replacable {
        tm.args.update_untracked(|args|
          if let DomTermArgs::Open(v) = args {
            let (index,sub) = match arg.index {
              either::Left(i) => ((i-1) as usize,None),
              either::Right((i,m)) => ((i-1) as usize,Some((m - 1) as usize))
            };
            if v.len() <= index { v.resize(index + 1, None); }
            let entry = &mut v[index];
            if let Some(sub) = sub {
              if let (_,either::Right(subs)) = entry.get_or_insert_with(|| (arg.mode,either::Right(Vec::new()))) {
                if subs.len() <= sub { subs.resize(sub + 1, None); }
                let entry = &mut subs[sub];
                *entry = Some(orig.html_string());
              } else {
                tracing::error!("{} is not a list",SHTMLKey::Arg.attr_name());
              }
            } else {
              *entry = Some((arg.mode,either::Left(orig.html_string())));
            }
          }
        )
      }
    }/*else {
      tracing::error!("{} outside of a term",SHTMLKey::Arg.attr_name());
    } */ 

  }

  let value : Option<InTermState> = None;
  let value_2 : Option<SkipOne> = None;
  view!{<Provider value><Provider value=value_2>{cont(orig)}</Provider></Provider>}
}