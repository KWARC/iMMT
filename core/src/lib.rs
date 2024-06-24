#![recursion_limit = "256"]

pub mod prelude {
    pub use crate::utils::filetree::{DirLike};
}
pub mod ontology {
    pub mod rdf;
    pub mod archives;
}

pub mod uris;
pub mod utils;

pub mod building {
    pub mod buildstate;
    pub mod formats;
}


pub mod content {
    mod constants;
    mod module;
    mod terms;
    pub use constants::*;
    pub use module::*;
    pub use terms::*;
}
pub mod narration {
    mod document;
    pub use document::*;
}

#[cfg(test)]
pub mod tests {
    pub use rstest::{fixture,rstest};
    pub use tracing::{info,warn,error};

    #[fixture]
    pub fn setup() {
        tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).try_init();
    }
}