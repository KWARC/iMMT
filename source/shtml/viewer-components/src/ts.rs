use std::marker::PhantomData;

use immt_ontology::uris::DocumentElementURI;
use wasm_bindgen::JsValue;
use web_sys::HtmlDivElement;

pub trait AsTs {
  fn as_ts(&self) -> JsValue;
}
pub trait FromTs:Sized {
  fn from_ts(v:JsValue) -> Result<Self,JsValue>;
}

impl AsTs for String {
  #[inline]
  fn as_ts(&self) -> JsValue {
      JsValue::from_str(self)
  }
}
impl FromTs for String {
  #[inline]
  fn from_ts(v:JsValue) -> Result<Self,JsValue> {
      v.as_string().map_or(Err(v),Ok)
  }
}

impl AsTs for DocumentElementURI {
  #[inline]
  fn as_ts(&self) -> JsValue {
      JsValue::from_str(self.to_string().as_str())
  }
}
impl FromTs for DocumentElementURI {
  #[inline]
  fn from_ts(v:JsValue) -> Result<Self,JsValue> {
      v.as_string().and_then(|s| s.parse().ok()).map_or(Err(v),Ok)
  }
}

impl AsTs for () {
  #[inline]
  fn as_ts(&self) -> JsValue {
      JsValue::NULL
  }
}
impl FromTs for () {
  #[inline]
  fn from_ts(v:JsValue) -> Result<Self,JsValue> {
      Ok(())
  }
}

impl AsTs for HtmlDivElement {
  fn as_ts(&self) -> JsValue {
      self.clone().into()
  }
}
impl FromTs for HtmlDivElement {
  fn from_ts(v:JsValue) -> Result<Self,JsValue> {
    use wasm_bindgen::JsCast;
    v.dyn_into()
  }
}

impl<T:AsTs> AsTs for Option<T> {
  #[inline]
  fn as_ts(&self) -> JsValue {
      match self {
          None => wasm_bindgen::JsValue::UNDEFINED,
          Some(v) => v.as_ts()
      }
  }
}
impl<T:FromTs> FromTs for Option<T> {
  #[inline]
  fn from_ts(v:JsValue) -> Result<Self,JsValue> {
      if v.is_null() || v.is_undefined() {
          Ok(None)
      } else {
          T::from_ts(v).map(Some)
      }
  }
}
pub trait JsFunArgable:Sized {
  fn call_js<R:FromTs>(&self,j:&JsFun<Self,R>) -> Result<JsValue,JsValue>;
}

impl<T:AsTs> JsFunArgable for T {
  #[inline]
  fn call_js<R:FromTs>(&self,j:&JsFun<Self,R>) -> Result<JsValue,JsValue> {
      j.js.call1(&JsValue::UNDEFINED,
          &self.as_ts()
      )
  }
}
impl<T1:AsTs,T2:AsTs> JsFunArgable for (T1,T2) {
  #[inline]
  fn call_js<R:FromTs>(&self,j:&JsFun<Self,R>) -> Result<JsValue,JsValue> {
      j.js.call2(&JsValue::UNDEFINED,
          &self.0.as_ts(),
          &self.1.as_ts()
      )
  }
}
impl<T1:AsTs,T2:AsTs,T3:AsTs> JsFunArgable for (T1,T2,T3) {
  #[inline]
  fn call_js<R:FromTs>(&self,j:&JsFun<Self,R>) -> Result<JsValue,JsValue> {
      j.js.call3(&JsValue::UNDEFINED,
          &self.0.as_ts(),
          &self.1.as_ts(),
          &self.2.as_ts()
      )
  }
}

pub struct JsFun<Args:JsFunArgable,R:FromTs> {
  pub js:send_wrapper::SendWrapper<web_sys::js_sys::Function>,
  ret:PhantomData<send_wrapper::SendWrapper<(Args,R)>>
}
// unsafe impl<Args:JsFunArgable,R:Tsable> Send for JsFun<Args,R> {}

impl<Args:JsFunArgable,R:FromTs> Clone for JsFun<Args,R> {
  #[inline]
  fn clone(&self) -> Self {
      Self {js:self.js.clone(),ret:PhantomData}
  }
}
impl<Args:JsFunArgable,R:FromTs> AsTs for JsFun<Args,R> {

  fn as_ts(&self) -> JsValue {
      (&*self.js).clone().into()
  }
}
impl<Args:JsFunArgable,R:FromTs> FromTs for JsFun<Args,R> {
  fn from_ts(v:JsValue) -> Result<Self,JsValue> {
      Ok(Self{js:send_wrapper::SendWrapper::new(web_sys::js_sys::Function::from(v)),ret:PhantomData})
  }
}

impl<Args:JsFunArgable,R:FromTs> JsFun<Args,R> {
  #[inline]
  pub fn apply(&self,args:&Args) -> Result<R,JsValue> {
      args.call_js(self).and_then(|r| R::from_ts(r))
  }
}

pub trait JsFunLike<Args:JsFunArgable,R:FromTs>:Fn(&Args) -> Result<R,String>+'static+Send+Sync {
  fn bx_clone(&self) -> Box<dyn JsFunLike<Args,R>>;
}
impl<Args:JsFunArgable,R:FromTs,T:Fn(&Args) -> Result<R,String>+Clone+'static+Send+Sync> JsFunLike<Args,R> for T {
  #[inline]
  fn bx_clone(&self) -> Box<dyn JsFunLike<Args,R>> {
    Box::new(self.clone())
  }
}

pub enum JsOrRsF<Args:JsFunArgable,R:FromTs> {
  Rs(Box<dyn JsFunLike<Args,R>>),
  Js(JsFun<Args,R>)
}
impl<Args:JsFunArgable+'static,R:FromTs+'static> Clone for JsOrRsF<Args,R> {
  #[inline]
  fn clone(&self) -> Self {
      match self {
        Self::Rs(s) => {
          let b = (&**s).bx_clone();
          Self::Rs(b)
        }
        Self::Js(j) => Self::Js(j.clone())
      }
  }
}
impl<Args:JsFunArgable,R:FromTs> JsOrRsF<Args,R> {
  #[inline]
  pub fn apply(&self,args:&Args) -> Result<R,String> {
    match self {
      Self::Rs(r) => r(args),
      Self::Js(j) => j.apply(args).map_err(|e| e.as_string().unwrap_or_default())
    }
  }
  #[inline]
  pub fn new(f:impl Fn(&Args) -> Result<R,String> + 'static + Clone + Send + Sync) -> Self {
    Self::Rs(Box::new(f))
  }
}
impl<Args:JsFunArgable,R:FromTs> From<JsFun<Args,R>> for JsOrRsF<Args,R>  {
  #[inline]
  fn from(value: JsFun<Args,R>) -> Self {
      Self::Js(value)
  }
}

impl<Args:JsFunArgable,R:FromTs> FromTs for JsOrRsF<Args,R> {
  fn from_ts(v:JsValue) -> Result<Self,JsValue> {
      let f: JsFun<Args,R> = JsFun::from_ts(v)?;
      Ok(Self::Js(f.into()))
  }
}

pub trait NamedJsFunction {
  type Args:JsFunArgable;
  type R:FromTs;
  type Base;

  #[cfg(feature="ts")]
  fn get(self) -> Self::Base;
}

#[macro_export]
macro_rules! ts_function {
  ($name:ident @ $ts_type:literal = $args:ty => $ret:ty) => {

      #[cfg(feature="ts")]
      #[::wasm_bindgen::prelude::wasm_bindgen]
      extern "C" {
        #[::wasm_bindgen::prelude::wasm_bindgen(extends = ::web_sys::js_sys::Function)]
        #[::wasm_bindgen::prelude::wasm_bindgen(typescript_type = $ts_type)]
        pub type $name;
      }


    #[cfg(not(feature="ts"))]
    #[derive(Clone)]
    pub struct $name;

    impl $crate::ts::NamedJsFunction for $name {
      type Args = $args;
      type R = $ret;
      type Base = JsFun<$args,$ret>;
      #[cfg(feature="ts")]
      fn get(self) -> Self::Base {
        JsFun {
          js:send_wrapper::SendWrapper::new(self.into()),
          ret:PhantomData
        }
      }
    }
    
  };
}

pub type TsCont = JsOrRsF<HtmlDivElement,()>;

impl TsCont {
  pub fn view(self) -> impl leptos::prelude::IntoView {
    use leptos::prelude::*;
    let ret = NodeRef::new();
    let _f = Effect::new(move || 
      if let Some(e) = ret.get() {
        if let Err(e) = self.apply(&e) {
          tracing::error!("Error calling continuation: {e}");
        }
      }
    );
    view!(<div node_ref = ret/>)
  }
  pub fn res_into_view(f:Result<Option<Self>,String>) -> impl leptos::prelude::IntoView {
    match f {
      Err(e) => {
        tracing::error!("Error getting continuation: {e}");
        None
      }
      Ok(None) => None,
      Ok(Some(f)) => Some(f.view())
    }
  }
}

ts_function!{
  SectionContinuation @ "(uri: string) => ( ((HTMLDivElement) => void) | undefined )"
  = DocumentElementURI => Option<TsCont>
}