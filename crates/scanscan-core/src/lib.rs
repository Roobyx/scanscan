//! scanscan core library: scanner, index, query engine, Docker collector,
//! snapshot catalog and the JSON-RPC daemon.

pub mod config;
pub mod daemon;
pub mod docker;
pub mod error;
pub mod index;
pub mod query;
pub mod scanner;
pub mod store;

pub use config::Config;
pub use error::{CoreError, Result};

/// Core crate version, taken from `Cargo.toml`.
pub const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");
