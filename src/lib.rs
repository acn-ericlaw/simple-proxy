//! simple-proxy: a minimalist Layer-4 TCP forwarder (Rust + Tokio).
//!
//! The binary (`src/main.rs`) is a thin CLI shell over these modules; they are exposed
//! here so the proxy can be embedded into a larger system without forking.
//!
//! ## Stable embedding API
//!
//! These modules form the stable public surface for embedding the forwarder:
//!
//! - [`config`] — parse and validate `proxy-config.json` ([`Config`](config::Config))
//! - [`proxy`] — high-level entry points ([`run_serve`](proxy::run_serve),
//!   [`run_forward`](proxy::run_forward)) plus the lower-level
//!   [`serve_listener`](proxy::serve_listener) / [`bind_reuse`](proxy::bind_reuse)
//!   used in integration tests
//! - [`relay`] — the bidirectional byte relay ([`relay`](relay::relay),
//!   [`RelayStats`](relay::RelayStats), [`ExitReason`](relay::ExitReason))
//! - [`shutdown`] — graceful shutdown primitives ([`channel`](shutdown::channel),
//!   [`ShutdownController`](shutdown::ShutdownController), [`Shutdown`](shutdown::Shutdown))
//! - [`allowlist`] — IP allow-list checking ([`is_authorized`](allowlist::is_authorized))
//!
//! ## Binary helpers (not part of the stable embedding contract)
//!
//! These modules support the `simple-proxy` CLI binary. Their interfaces may change
//! between minor versions:
//!
//! - [`cli`] — command-line argument parsing
//! - [`discovery`] — shell-command-based target IP resolution
//! - [`log`] — internal `logln!` macro

// Stable embedding API
pub mod allowlist;
pub mod config;
pub mod proxy;
pub mod relay;
pub mod shutdown;

// Binary helpers
pub mod cli;
pub mod discovery;
pub mod log;
