use std::path::PathBuf;
use lazy_static::lazy_static;
pub static API_VERSION: &str = env!("CARGO_PKG_VERSION");
pub static RUSTC_VERSION: &str = env!("RUSTC_VERSION");

pub mod backend {
    pub mod archives;
    pub mod manager;
    #[cfg(feature="oxigraph")]
    pub mod relational;
}
pub mod extensions;
pub mod controller;
pub mod building {
    pub mod targets;
    pub mod buildqueue;
}
pub mod utils {
    use lazy_static::lazy_static;
    use immt_core::utils::triomphe::Arc;

    pub mod asyncs;
    pub mod circular_buffer;
    
    lazy_static!(
        pub static ref EMPTY_ARCSTR: Arc<str> = Arc::from("");
    );
    
    pub fn time<A,F:FnOnce() -> A>(f:F,s:&str) -> A {
        let start = std::time::Instant::now();
        let ret = f();
        let dur = start.elapsed();
        tracing::info!("{s} took {:?}", dur);
        ret
    }
}

pub mod core { pub use immt_core::*; }

#[cfg(feature = "rayon")]
pub mod par {
    pub use spliter::ParallelSpliterator;
    pub use rayon::iter::{IntoParallelIterator,ParallelIterator};
}

lazy_static! {
    pub static ref MATHHUB_PATHS: Box<[PathBuf]> = mathhubs().into();
}

fn mathhubs() -> Vec<PathBuf> {
    if let Ok(f) = std::env::var("MATHHUB") {
        return f.split(',').map(|s| PathBuf::from(s.trim())).collect()
    }
    if let Some(d) = simple_home_dir::home_dir() {
        let p = d.join(".mathhub").join("mathhub.path");
        if let Ok(f) = std::fs::read_to_string(p) {
            return f.split('\n').map(|s| PathBuf::from(s.trim())).collect()
        }
        return vec![d.join("MathHub")];
    }
    panic!(
        "No MathHub directory found and default ~/MathHub not accessible!\n\
    Please set the MATHHUB environment variable or create a file ~/.mathhub/mathhub.path containing \
    the path to the MathHub directory."
    )
}

#[cfg(test)]
pub mod tests {
    pub use rstest::{fixture,rstest};
    pub use tracing::{info,warn,error};

    #[fixture]
    pub fn setup() {
        let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).try_init();
    }
}