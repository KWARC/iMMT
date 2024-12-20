#![allow(clippy::must_use_candidate)]

mod r#await;
mod binder;
mod popover;
mod trees;
mod drawer;
mod spinner;

#[cfg(feature="hydrate")]
mod errors;
#[cfg(feature="hydrate")]
pub use errors::*;

#[cfg(any(feature="ssr",feature="hydrate"))]
pub use theming::*;
#[cfg(any(feature="ssr",feature="hydrate"))]
mod theming;
mod anchors;
mod block;

#[cfg(not(any(feature="ssr",feature="hydrate")))]
#[component(transparent)]
pub fn Themer<Ch:IntoView+'static>(children:TypedChildren<Ch>) -> impl IntoView {
    use thaw::{ConfigProvider,ToasterProvider,Theme};
    let children = children.into_inner();
    view!{
      <ConfigProvider>
        {children()}
        //<ToasterProvider>{children()}</ToasterProvider>
      </ConfigProvider>
    }
}


pub use popover::*;
pub use r#await::*;
pub use trees::*;
pub use drawer::*;
pub use anchors::*;
pub use spinner::*;
pub use block::*;

#[leptos::prelude::slot]
pub struct Header { children:leptos::prelude::Children }
#[leptos::prelude::slot]
pub struct Trigger { children:leptos::prelude::Children }

use leptos::{either::Either, prelude::*};

use crate::inject_css;

#[component]
pub fn Collapsible<Ch:IntoView+'static>(
    #[prop(optional)] lazy:bool,
    #[prop(optional)] header:Option<Header>,
    children:TypedChildrenMut<Ch>
) -> impl IntoView {
  let mut children = children.into_inner();
    let expanded = RwSignal::new(false);
    view!{<details>
        <summary on:click=move |_| expanded.update(|b| *b = !*b)>{
            header.map(|c| (c.children)())
        }</summary>
        <div>{
            if lazy { Either::Left(move || if expanded.get() {
                Some(children())
                } else { None }
            )} else {Either::Right(children())}
        }</div>
    </details>}
}

#[component]
pub fn Burger<Ch:IntoView+'static>(children:TypedChildren<Ch>) -> impl IntoView {
  use thaw::{Menu,MenuTriggerType,MenuTrigger};
  use icondata_ch::ChMenuHamburger;
  let children = children.into_inner();
  inject_css("immt-burger", ".immt-burger {position:absolute !important;right:-10px;}");
  view!{<div style="position:fixed;right:10px;position-anchor:inherit;">
    <Menu class="immt-burger" on_select=|_| () trigger_type=MenuTriggerType::Hover>
        <MenuTrigger slot><div><thaw::Icon width="2.5em" height="2.5em" icon=ChMenuHamburger/></div></MenuTrigger>
        {children()}
    </Menu>
  </div>}
}