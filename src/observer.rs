//! Control-plane lifecycle hook.
//!
//! A [`ConnObserver`] receives a [`ConnEvent`] at each point in a connection's life
//! (rejected / upstream-unavailable / opened / closed). It is invoked from the
//! per-connection task **around** the byte relay — never in the data path — so it is a
//! pure *control-plane* signal. The byte forwarding in [`crate::relay`] is unaffected.
//!
//! The default ([`NoopObserver`]) does nothing and pulls in no dependencies, so the
//! `simple-proxy` binary stays dependency-light. Embedders (or the
//! `event_bus_signaling` example) supply their own observer to bridge these events into
//! metrics, an event bus, tracing, etc.

use std::net::SocketAddr;

/// A control-plane lifecycle event for a single inbound connection.
///
/// Marked `#[non_exhaustive]` so new events can be added without breaking downstream
/// `match` arms.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum ConnEvent {
    /// An inbound connection was dropped by the IP allow-list before any forwarding.
    Rejected {
        peer: SocketAddr,
        target: SocketAddr,
    },
    /// An authorized client connected, but dialing the upstream failed (or timed out).
    UpstreamUnavailable {
        session: String,
        peer: SocketAddr,
        target: SocketAddr,
    },
    /// The upstream is connected and the relay is now active ("socket open").
    Opened {
        session: String,
        peer: SocketAddr,
        target: SocketAddr,
    },
    /// The relay finished ("socket close"), with byte counts and why it ended
    /// (see [`crate::relay::ExitReason::label`]).
    Closed {
        session: String,
        peer: SocketAddr,
        target: SocketAddr,
        rx: u64,
        tx: u64,
        reason: &'static str,
    },
}

/// Receives [`ConnEvent`]s emitted by the proxy's connection lifecycle.
///
/// Implementations must be cheap and non-blocking: `on_event` runs on the connection
/// task, off the data path, so blocking here would stall connection setup/teardown
/// (not the byte relay, but still the accepting task's bookkeeping).
pub trait ConnObserver: Send + Sync {
    fn on_event(&self, event: ConnEvent);
}

/// A [`ConnObserver`] that ignores every event — the default used by the CLI binary so
/// it carries no observability dependencies.
pub struct NoopObserver;

impl ConnObserver for NoopObserver {
    #[inline]
    fn on_event(&self, _event: ConnEvent) {}
}
