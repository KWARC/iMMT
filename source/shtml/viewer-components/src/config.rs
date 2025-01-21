use immt_ontology::uris::{DocumentElementURI, URI};
use leptos::prelude::*;
use shtml_extraction::open::terms::VarOrSym;

#[derive(Debug,Clone)]
pub(crate) struct IdPrefix(pub String);
impl IdPrefix {
    pub fn new_id(self,s:&str) -> String {
        if self.0.is_empty() {
            s.to_string()
        } else {
            format!("{}/{s}",self.0)
        }
    }
}

#[derive(Clone)]
pub(crate) struct SHTMLConfig {
  owner: Owner,
  on_clicks:StoredValue<immt_utils::prelude::HMap<VarOrSym,RwSignal<bool>>>,
  #[cfg(feature="omdoc")]
  forced_notations: StoredValue<immt_utils::prelude::HMap<URI,RwSignal<Option<DocumentElementURI>>>>
}

impl SHTMLConfig {

  pub fn new() -> Self {
    let owner = Owner::new();//current().expect("Something went horribly wrong");
    Self {
        owner,
        on_clicks:StoredValue::new(immt_utils::prelude::HMap::default()),
        #[cfg(feature="omdoc")]
        forced_notations:StoredValue::new(immt_utils::prelude::HMap::default())
    }
  }

  pub fn do_in<R>(&self,f:impl FnOnce() -> R) -> R {
      self.owner.clone().with(f)
  }

  pub fn get_on_click(&self,uri:&VarOrSym) -> RwSignal<bool> {
    use thaw::{Dialog,DialogSurface};
    use thaw::{Combobox,ComboboxOption,ComboboxOptionGroup,Divider};
    use crate::components::terms::do_onclick;
    if let Some(s) = self.on_clicks.with_value(|map| map.get(uri).copied()) {
        return s
    }
    self.owner.with(move || {
        let signal = RwSignal::new(false);
        let uri = uri.clone();
        self.on_clicks.update_value(|map| {map.insert(uri.clone(),signal);});
        let _ = view!{<Dialog open=signal><DialogSurface>{
                do_onclick(uri)
        }</DialogSurface></Dialog>};
        signal
    })
  }
}

#[cfg(feature="omdoc")]
impl SHTMLConfig {

  pub fn get_forced_notation(&self,uri:&URI) -> RwSignal<Option<DocumentElementURI>> {
    self.owner.with(||
        self.forced_notations
            .with_value(|map| map.get(uri).copied())
            .unwrap_or_else(|| {
                #[cfg(any(feature="csr",feature="hydrate"))]
                let sig = {
                    use gloo_storage::Storage;
                    let s = gloo_storage::LocalStorage::get(format!("notation_{uri}"))
                        .map_or_else(
                        |_| RwSignal::new(None),
                        |v:DocumentElementURI| {
                            let uri = uri.clone();
                            let sig = RwSignal::new(None);
                            let _ = Resource::new(|| (),move |()| {let uri = uri.clone(); let v = v.clone();async move {
                                let _ = crate::remote::server_config.notations(uri).await;
                                sig.set(Some(v));
                            }}
                            );
                            sig
                        }
                    );
                    let uri = uri.clone();
                    Effect::new(move || {
                        s.with(|s|
                            if let Some(s) = s.as_ref() {
                                let _ = gloo_storage::LocalStorage::set(format!("notation_{uri}"),&s);
                            } else {
                                let _ = gloo_storage::LocalStorage::delete(format!("notation_{uri}"));
                            }
                        );
                    });
                    s
                };
                #[cfg(not(any(feature="csr",feature="hydrate")))]
                let sig = RwSignal::new(None);
                self.forced_notations.update_value(|map| {map.insert(uri.clone(),sig);});
                sig
            }
        )
    )
  }
}