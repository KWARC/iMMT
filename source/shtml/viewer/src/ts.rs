#![allow(non_local_definitions)]

use immt_utils::vecmap::VecMap;
use shtml_viewer_components::{components::{documents::{DocumentFromURI, DocumentString, FragmentFromURI, FragmentString, FragmentStringProps}, TOCElem, TOCSource}, ts::{LeptosContext, NamedJsFunction, SectionContinuation, TsCont, TsTopCont}, ExerciseOptions};
use immt_ontology::{narration::exercises::{ExerciseFeedback, ExerciseResponse, Solutions}, uris::{DocumentElementURI, DocumentURI}};
use wasm_bindgen::prelude::wasm_bindgen;
use leptos::{context::Provider, either::Either, prelude::*};

#[wasm_bindgen]
/// activates debug logging
pub fn set_debug_log() {
    let _ = tracing_wasm::try_set_as_global_default();
}

#[derive(Debug,Clone,serde::Serialize,serde::Deserialize,tsify_next::Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
/// Options for rendering an SHTML document
/// - `FromBackend`: calls the backend for the document
///     uri: the URI of the document (as string)
///     toc: if defined, will render a table of contents for the document
// - Prerendered: Take the existent DOM HTMLElement as is
/// - `HtmlString`: render the provided HTML String
///     html: the HTML String
///     toc: if defined, will render a table of contents for the document
pub enum DocumentOptions {
    FromBackend {
        #[tsify(type = "string")]
        uri:DocumentURI,
        toc:Option<TOCOptions>
    },
    //Prerendered,
    HtmlString {
        html:String,
        toc:Option<Vec<TOCElem>>
    }
}

#[derive(Debug,Clone,serde::Serialize,serde::Deserialize,tsify_next::Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
/// Options for rendering an SHTML document fragment
/// - `FromBackend`: calls the backend for the document fragment
///     uri: the URI of the document fragment (as string)
// - Prerendered: Take the existent DOM HTMLElement as is
/// - `HtmlString`: render the provided HTML String
///     html: the HTML String
pub enum FragmentOptions {
    FromBackend {
        #[tsify(type = "string")]
        uri:DocumentElementURI,
    },
    //Prerendered,
    HtmlString {
      #[tsify(type = "string | undefined")]
        uri:Option<DocumentElementURI>,
        html:String,
    }
}


#[derive(Debug,Clone,serde::Serialize,serde::Deserialize,tsify_next::Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
/// Options for rendering a table of contents
/// `FromBackend` will retrieve it from the remote backend
/// `Predefined(toc)` will render the provided TOC
pub enum TOCOptions {
    FromBackend,
    Predefined(Vec<TOCElem>)
}

impl From<TOCOptions> for TOCSource {
  fn from(value: TOCOptions) -> Self {
      match value {
          TOCOptions::FromBackend =>Self::Get,
          TOCOptions::Predefined(toc) => Self::Ready(toc)
      }
  }
}

#[allow(clippy::needless_pass_by_value)]
#[wasm_bindgen]
/// render an SHTML document to the provided element
/// #### Errors
pub fn render_document(
  to:leptos::web_sys::HtmlElement,
  document:DocumentOptions,
  on_section_start: Option<SectionContinuation>,
  on_section_end: Option<SectionContinuation>,
  context:Option<LeptosContext>
) -> Result<SHTMLMountHandle,String> {
  fn inner(
    to:leptos::web_sys::HtmlElement,
    document:DocumentOptions,
    on_section_start: Option<SectionContinuation>,
    on_section_end: Option<SectionContinuation>
  ) -> Result<SHTMLMountHandle,String> {
    use shtml_viewer_components::SHTMLGlobalSetup;
    use immt_web_utils::components::Themer;

    let comp = move || match document {
      DocumentOptions::HtmlString{html,toc} => {
        let toc = toc.map_or(TOCSource::None,TOCSource::Ready);
        Either::Left(view!{<GlobalSetup><DocumentString html toc/></GlobalSetup>})
      }
      DocumentOptions::FromBackend{uri,toc} => {
        let toc = toc.map_or(TOCSource::None,Into::into);
        Either::Right(view!{<GlobalSetup><DocumentFromURI uri toc/></GlobalSetup>})
      }
    };

    Ok(SHTMLMountHandle::new(to,move || {
      if let Some(start) = on_section_start {
        shtml_viewer_components::components::OnSectionBegin::set(start.get().into());
      };
      if let Some(end) = on_section_end {
        shtml_viewer_components::components::OnSectionEnd::set(end.get().into());
      };
      comp()
    }))
  }
  if let Some(context) = context {
    context.with(move || inner(to,document,on_section_start,on_section_end))
  } else {
    inner(to,document,on_section_start,on_section_end)
  }
}

shtml_viewer_components::ts_function!{
  ExerciseCont @ "(r:ExerciseResponse) => void"
  = (ExerciseResponse) => ()
}


#[derive(Debug,Clone,serde::Serialize,serde::Deserialize,tsify_next::Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub enum ExerciseOption {
  //OnResponse( #[tsify(type = "(r:ExerciseResponse) => void")] ExerciseCont),
  WithFeedback(#[tsify(type = "[string,ExerciseFeedback][]")] Vec<(DocumentElementURI,ExerciseFeedback)>),
  WithSolutions(#[tsify(type = "[string,Solutions][]")] Vec<(DocumentElementURI,Solutions)>)
}
impl Into<ExerciseOptions> for ExerciseOption {
  fn into(self) -> ExerciseOptions {
    match self {
      //Self::OnResponse(f) => ExerciseOptions::OnResponse(f.get().into()),
      Self::WithFeedback(v) => ExerciseOptions::WithFeedback(v.into()),
      Self::WithSolutions(s) => ExerciseOptions::WithSolutions(s.into())
    }
  }
}


#[allow(clippy::needless_pass_by_value)]
#[wasm_bindgen]
/// render an SHTML document fragment to the provided element
/// #### Errors
pub fn render_fragment_with_cont(
  to:leptos::web_sys::HtmlElement,
  fragment:FragmentOptions,
  context:Option<LeptosContext>,
  exercise_cont:ExerciseCont
) -> Result<SHTMLMountHandle,String> {
  let cont = ExerciseOptions::OnResponse(exercise_cont.get().into());
  fn inner(
    to:leptos::web_sys::HtmlElement,
    fragment:FragmentOptions,
    cont:ExerciseOptions
  ) -> Result<SHTMLMountHandle,String> {
    use shtml_viewer_components::SHTMLGlobalSetup;
    use immt_web_utils::components::Themer;

    let comp = move || match fragment {
      FragmentOptions::HtmlString{html,uri} => {
        Either::Left(FragmentString(FragmentStringProps {html,uri}))
      }
      FragmentOptions::FromBackend{uri} => {
        Either::Right(view!{<FragmentFromURI uri/>})
      }
    };

    Ok(SHTMLMountHandle::new(to,move || view!{
      <GlobalSetup><Provider value=cont>{comp()}</Provider></GlobalSetup>
    }))
  }
  if let Some(context) = context {
    context.with(move || inner(to,fragment,cont))
  } else {
    inner(to,fragment,cont)
  }
}

#[allow(clippy::needless_pass_by_value)]
#[wasm_bindgen]
/// render an SHTML document fragment to the provided element
/// #### Errors
pub fn render_fragment(
  to:leptos::web_sys::HtmlElement,
  fragment:FragmentOptions,
  context:Option<LeptosContext>,
  exercise_options:Option<ExerciseOption>
) -> Result<SHTMLMountHandle,String> {
  fn inner(
    to:leptos::web_sys::HtmlElement,
    fragment:FragmentOptions,
    exercise_options:Option<ExerciseOption>
  ) -> Result<SHTMLMountHandle,String> {
    use shtml_viewer_components::SHTMLGlobalSetup;
    use immt_web_utils::components::Themer;

    let comp = move || match fragment {
      FragmentOptions::HtmlString{html,uri} => {
        Either::Left(FragmentString(FragmentStringProps {html,uri}))
      }
      FragmentOptions::FromBackend{uri} => {
        Either::Right(view!{<FragmentFromURI uri/>})
      }
    };

    let comp = move || if let Some(opt) = exercise_options {
      let opt: ExerciseOptions = opt.into();
      Either::Left(view!{<GlobalSetup><Provider value=opt>{comp()}</Provider></GlobalSetup>})
    } else {
      Either::Right(view!{<GlobalSetup>{comp()}</GlobalSetup>})
    };

    Ok(SHTMLMountHandle::new(to,move || comp()))
  }
  if let Some(context) = context {
    context.with(move || inner(to,fragment,exercise_options))
  } else {
    inner(to,fragment,exercise_options)
  }
}

#[wasm_bindgen]
/// sets up a leptos context for rendering SHTML documents or fragments.
/// If a context already exists, does nothing, so is cheap to call
/// [render_document] and [render_fragment] also inject a context
/// iff none already exists, so this is optional in every case.
pub fn shtml_setup(to:leptos::web_sys::HtmlElement,cont:TsTopCont) -> SHTMLMountHandle {
  let cont = cont.to_cont();
  SHTMLMountHandle::new(to,move || view! {
    <GlobalSetup>{cont.view()}</GlobalSetup>
  })
}

#[component]
fn GlobalSetup<V:IntoView+'static>(children:TypedChildren<V>) -> impl IntoView {
  use shtml_viewer_components::SHTMLGlobalSetup;
  use immt_web_utils::components::Themer;
  use leptos::either::Either::{Left,Right};
  let children = children.into_inner();
  let children = move || if shtml_viewer_components::is_in_shtml() {
    Left(children())
  } else {
    Right(view!(<SHTMLGlobalSetup>{children()}</SHTMLGlobalSetup>))
  };
  if with_context::<thaw::ConfigInjection,_>(|_| ()).is_none() {
    Left(view!(<Themer>{children()}</Themer>))
  } else {
    Right(children())
  }
}

#[wasm_bindgen]
pub struct SHTMLMountHandle{
  mount:std::cell::Cell<Option<leptos::prelude::UnmountHandle<leptos::tachys::view::any_view::AnyViewState>>>
}

#[wasm_bindgen]
impl SHTMLMountHandle {
  /// unmounts the view and cleans up the reactive system.
  /// Not calling this is a memory leak
  pub fn unmount(&self) {
    if let Some(mount) = self.mount.take() {
      drop(mount);
    }
  }
  fn new<V:IntoView+'static>(div:leptos::web_sys::HtmlElement,f:impl FnOnce() -> V + 'static) -> Self {
    let handle = leptos::prelude::mount_to(div,move || {
      f().into_any()
    });
    Self {
      mount:std::cell::Cell::new(Some(handle))
    }
  }
}

pub mod server {
  use wasm_bindgen::prelude::wasm_bindgen;
  use shtml_viewer_components::remote::{ServerConfig,server_config};
  pub use immt_utils::CSS;
  use tsify_next::Tsify;
  /// The currently set server URL
  #[wasm_bindgen]
  pub fn get_server_url() -> String {
    server_config.server_url.lock().clone()
  }

  #[derive(Tsify, serde::Serialize, serde::Deserialize)]
  #[tsify(into_wasm_abi, from_wasm_abi)]
  pub struct HTMLFragment {
    pub css: Vec<CSS>,
    pub html: String
  }

  #[wasm_bindgen]
  /// #### Errors
  pub async fn get_document_html(doc:&str) -> Result<HTMLFragment,String> { 
    let doc = doc.parse().map_err(|e| "invalid document URI".to_string())?;
    server_config.inputref(doc).await.map(|(_,css,html)|
      HTMLFragment {css, html}
    )
  }

  #[wasm_bindgen]
  /// #### Errors
  pub async fn get_paragraph_html(elem:&str) -> Result<HTMLFragment,String> { 
    let doc = elem.parse().map_err(|e| "invalid document URI".to_string())?;
    server_config.paragraph(doc).await.map(|(_,css,html)|
      HTMLFragment {css, html}
    )
  }

}