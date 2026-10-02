//! scanscan core library: scanner, index, query engine and Docker collector.
//!
//! Phase 0 establishes the crate surface and the filesystem abstraction. The
//! streaming scanner, CAS block store and query engine land in Phase 1.

pub mod config;
pub mod error;
pub mod scanner;

pub use config::Config;
pub use error::{CoreError, Result};

/// Core crate version, taken from `Cargo.toml`.
pub const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");
