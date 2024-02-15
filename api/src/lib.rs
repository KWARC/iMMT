#![recursion_limit="256"]

use std::path::PathBuf;
//use std::rc::Rc;
use std::sync::Arc;

//#[cfg(debug_assertions)]
//pub type Str = String;
//#[cfg(not(debug_assertions))]
pub type Str = Arc<str>;

//#[cfg(debug_assertions)]
//pub type Seq<A> = Vec<A>;
//#[cfg(not(debug_assertions))]
pub type Seq<A> = Arc<[A]>;

pub mod ontology {
    pub mod rdf;
}

pub mod formats;
pub mod source_files;
pub mod uris;
pub mod archives;

pub mod utils {
    use crate::Str;

    pub mod parsing;
    pub mod iter;
    pub mod circular_buffer;

    pub type HMap<A,B> = ahash::HashMap<A,B>;

    pub type MMTURI = Str;
}

#[cfg(feature = "fs")]
pub fn mathhub() -> PathBuf {
    if let Ok(f) = std::env::var("MATHHUB") {
       return PathBuf::from(f);
    }
    if let Some(d) = simple_home_dir::home_dir() {
        let p = d.join(".stex").join("mathhub.path");
        if let Ok(f) = std::fs::read_to_string(p) {
            return PathBuf::from(f);
        }
        return d.join("MathHub");
    }
    panic!("No MathHub directory found and default ~/MathHub not accessible!\n\
    Please set the MATHHUB environment variable or create a file ~/.stex/mathhub.path containing \
    the path to the MathHub directory.")
}