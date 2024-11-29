/*! Foo Bar
 * 
 * See [endpoints] for public API endpoints
*/
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(all(feature = "ssr", feature = "hydrate", not(doc)))]
compile_error!("features \"ssr\" and \"hydrate\" cannot be enabled at the same time");

#[cfg(feature = "ssr")]
pub mod server;

pub mod router;
pub mod users;
pub mod utils;

pub(crate) mod fns {
    use std::{future::Future, pin::Pin};

    use immt_ontology::{languages::Language, narration::LOKind, uris::{ArchiveId, DocumentElementURI, DocumentURI, SymbolURI, URI}};
    use immt_utils::CSS;
    use leptos::prelude::ServerFnError;
    use shtml_viewer_components::components::{omdoc::AnySpec, TOCElem};

    fn fragment(uri:Option<URI>,rp:Option<String>,a:Option<ArchiveId>,p:Option<String>,l:Option<Language>,d:Option<String>,e:Option<String>,m:Option<String>,s:Option<String>)
    -> Pin<Box<dyn Future<Output=Result<(Vec<CSS>,String),ServerFnError<String>>> + Send>> {
        Box::pin(crate::router::content::fragment(uri,rp,a,p,l,d,e,m,s))
    }
    fn full_doc(uri:Option<DocumentURI>,rp:Option<String>,a:Option<ArchiveId>,p:Option<String>,l:Option<Language>,d:Option<String>)
    -> Pin<Box<dyn Future<Output=Result<(DocumentURI,Vec<CSS>,String),ServerFnError<String>>> + Send>> {
        Box::pin(crate::router::content::document(uri,rp,a,p,l,d))
    }
    fn toc(uri:Option<DocumentURI>,rp:Option<String>,a:Option<ArchiveId>,p:Option<String>,l:Option<Language>,d:Option<String>)
    -> Pin<Box<dyn Future<Output=Result<(Vec<CSS>,Vec<TOCElem>),ServerFnError<String>>> + Send>> {
        Box::pin(crate::router::content::toc(uri,rp,a,p,l,d))
    }
    fn los(uri:Option<SymbolURI>,a:Option<ArchiveId>,p:Option<String>,l:Option<Language>,m:Option<String>,s:Option<String>)
    -> Pin<Box<dyn Future<Output=Result<Vec<(DocumentElementURI,LOKind)>,ServerFnError<String>>> + Send>> {
        Box::pin(crate::router::content::los(uri,a,p,l,m,s))
    }
    fn omdoc(uri:Option<URI>,rp:Option<String>,a:Option<ArchiveId>,p:Option<String>,l:Option<Language>,d:Option<String>,e:Option<String>,m:Option<String>,s:Option<String>)
    -> Pin<Box<dyn Future<Output=Result<(Vec<CSS>,AnySpec),ServerFnError<String>>> + Send>> {
        Box::pin(crate::router::content::omdoc(uri,rp,a,p,l,d,e,m,s))
    }
    pub(super) fn init() {
        shtml_viewer_components::config::ServerConfig::initialize(
            fragment,full_doc,toc,omdoc,los
        );
    }
}

#[cfg(feature = "hydrate")]
#[leptos::wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    //use router::*;
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();
    fns::init();
    leptos::mount::hydrate_body(router::Main);
}

#[cfg(doc)]
pub mod endpoints;