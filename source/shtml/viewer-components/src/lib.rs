#![allow(clippy::must_use_candidate)]
#![allow(clippy::module_name_repetitions)]

//mod popover;

mod extractor;
pub mod components;
pub mod config;

use components::{inputref::InInputRef, SHTMLComponents};
use immt_utils::prelude::HMap;
use leptos::prelude::*;
use shtml_extraction::prelude::*;
use leptos::tachys::view::any_view::AnyView;
use leptos::web_sys::Element;
use extractor::DOMExtractor;
use crate::extractor::NodeAttrs;
use immt_ontology::uris::{DocumentURI, NarrativeURI};



#[derive(Debug,Clone)]
struct IdPrefix(pub String);
impl IdPrefix {
    pub fn new_id(self,s:&str) -> String {
        if self.0.is_empty() {
            s.to_string()
        } else {
            format!("{}/{s}",self.0)
        }
    }
}


#[cfg(any(feature="ssr",feature="hydrate"))]
#[cfg_attr(any(feature="ssr",feature="hydrate"),component)]
pub fn SHTMLDocument(uri:DocumentURI, children: Children, #[prop(optional)] on_load:Option<RwSignal<bool>>) -> impl IntoView {
    do_document(uri,children, on_load)
}

#[cfg(feature="csr")]
#[cfg_attr(feature="csr",component)]
pub fn SHTMLDocument(children: Children, #[prop(optional)] on_load:Option<RwSignal<bool>>) -> impl IntoView {
    let uri = "http://unknown.document?a=no/archive&d=unknown_document&l=en".parse().unwrap_or_else(|_| unreachable!());
    do_document(uri,children, on_load)
}

fn do_document(uri:DocumentURI,children:Children, on_load:Option<RwSignal<bool>>) -> impl IntoView {
    use crate::components::navigation::{Nav,NavElems,URLFragment};
    //let config = config::ServerConfig::clone_static();
    //provide_context(config);
    #[cfg(any(feature="csr",feature="hydrate"))]
    provide_context(RwSignal::new(DOMExtractor::default()));
    provide_context(InInputRef(false));
    provide_context(RwSignal::new(NavElems{ids:HMap::default()}));
    provide_context(IdPrefix(String::new()));
    provide_context(URLFragment::new());
    provide_context(NarrativeURI::Document(uri));
    let r = children();
    view! {
        <Nav on_load/>
        {r}
    }
}

pub static RULES:[SHTMLExtractionRule<DOMExtractor>;23] = [
    SHTMLTag::Section.rule(),
    SHTMLTag::Term.rule(),
    SHTMLTag::Arg.rule(),

    SHTMLTag::InputRef.rule(),


    SHTMLTag::Comp.rule(),
    SHTMLTag::VarComp.rule(),
    SHTMLTag::MainComp.rule(),

    SHTMLTag::IfInputref.rule(),

    // ---- no-ops --------
    SHTMLTag::ArgMode.rule(),
    SHTMLTag::NotationId.rule(),
    SHTMLTag::Head.rule(),
    SHTMLTag::Language.rule(),
    SHTMLTag::Metatheory.rule(),
    SHTMLTag::Signature.rule(),
    SHTMLTag::Args.rule(),
    SHTMLTag::Macroname.rule(),
    SHTMLTag::Inline.rule(),
    SHTMLTag::Fors.rule(),
    SHTMLTag::Id.rule(),
    SHTMLTag::NotationFragment.rule(),
    SHTMLTag::Precedence.rule(),
    SHTMLTag::Role.rule(),
    SHTMLTag::Argprecs.rule()
];

#[cfg_attr(not(feature="inline-js"),wasm_bindgen::prelude::wasm_bindgen)]
#[cfg_attr(feature="inline-js",wasm_bindgen::prelude::wasm_bindgen(inline_js = "
export function hasShtmlAttribute(node) {
    const attributes = node.attributes;
    for (let i = 0; i < attributes.length; i++) {
        if (attributes[i].name.startsWith('shtml:')) {
            return true;
        }
    }
    return false;
}

export function getDataShtmlAttributes(node) {
    const result = [];
    const attributes = node.attributes;
    for (let i = 0; i < attributes.length; i++) {
        if (attributes[i].name.startsWith('shtml:')) {
            result.push(attributes[i].name);
        }
    }
    return result;
}
"))]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = "hasShtmlAttribute")]
    fn has_shtml_attribute(node: &leptos::web_sys::Node) -> bool;
}

#[allow(clippy::missing_const_for_fn)]
#[allow(unreachable_code)]
#[allow(clippy::needless_return)]
pub fn iterate(e:&Element) -> Option<AnyView<Dom>> {
    tracing::trace!("iterating {} ({:?})",e.outer_html(),std::thread::current().id());
    #[cfg(any(feature="csr",feature="hydrate"))]
    {
        if !has_shtml_attribute(e) {
            tracing::trace!("No attributes");
            return None
        }
        tracing::trace!("Has shtml attributes");
        let sig = expect_context::<RwSignal<DOMExtractor>>();
        let r = sig.update_untracked(|extractor| {
            let mut attrs = NodeAttrs::new(e);
            RULES.applicable_rules(extractor,&mut attrs)
        });
        return r.map(|elements| {
            tracing::trace!("got elements: {elements:?}");
            let in_math = immt_web_utils::mathml::is(&e.tag_name()).is_some();
            let orig = e.clone().into();
            view!(<SHTMLComponents orig elements in_math/>).into_any()
        })
    }
    #[cfg(not(any(feature="csr",feature="hydrate")))]
    {None}
}