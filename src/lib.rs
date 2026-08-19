mod rendezvous_server;
pub use rendezvous_server::*;
pub mod common;
mod database;
#[cfg(test)]
mod dynamic_boundaries;
mod peer;
mod version {
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");
}
