//! simple-proxy: a minimalist Layer-4 TCP forwarder (Rust + Tokio).
//!
//! The binary (`src/main.rs`) is a thin CLI shell over these modules; they are exposed
//! here so integration tests can drive the proxy in-process.

pub mod allowlist;
pub mod cli;
pub mod config;
pub mod discovery;
pub mod log;
pub mod proxy;
pub mod relay;
pub mod shutdown;
