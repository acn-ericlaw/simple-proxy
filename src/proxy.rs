//! Listener orchestration: bind, gate inbound connections, dial the upstream, and
//! hand each accepted socket to [`crate::relay`].

use crate::allowlist::is_authorized;
use crate::config::Config;
use crate::discovery;
use crate::log::{group, session_id};
use crate::logln;
use crate::observer::{ConnEvent, ConnObserver, NoopObserver};
use crate::relay::{relay, ExitReason};
use crate::shutdown::Shutdown;
use anyhow::{Context, Result};
use std::io::ErrorKind;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpSocket, TcpStream};

/// How long to wait for an upstream connect before treating it as a dead target.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Idle timeout for the one-shot `forward` mode (the daemon reads its own from config).
const FORWARD_IDLE: Duration = Duration::from_secs(1800);

/// Run the config-driven daemon: resolve the target, then forward every port pair.
pub async fn run_serve(cfg: Config, shutdown: Shutdown) -> Result<()> {
    let target_ip: IpAddr = match &cfg.discovery {
        Some(d) => {
            logln!(
                "Resolving target IP using command {:?} (tag {:?}, index {})",
                d.command,
                d.tag,
                d.index
            );
            let ip = discovery::resolve(&d.command, &d.tag, d.index)
                .await
                .context("resolving target IP via discovery command")?;
            logln!("Resolved target IP {ip}");
            ip
        }
        // Safe to unwrap: validated as a parseable IP at config load time.
        None => cfg.target_ip.as_ref().unwrap().parse().unwrap(),
    };

    logln!("Authorized users {:?}", cfg.authorized);

    let allowlist = Arc::new(cfg.authorized.clone());
    let conns = Arc::new(AtomicUsize::new(0));
    let idle = cfg.idle_timeout();

    let mut handles = Vec::with_capacity(cfg.source_ports.len());
    for (i, &source_port) in cfg.source_ports.iter().enumerate() {
        let target_port = cfg.target_ports[i];
        let source = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), source_port);
        let target = SocketAddr::new(target_ip, target_port);
        handles.push(tokio::spawn(run_forwarder(
            source,
            target,
            Some(allowlist.clone()),
            cfg.restart,
            idle,
            conns.clone(),
            shutdown.clone(),
        )));
    }

    for h in handles {
        let _ = h.await; // each forwarder logs its own errors; never abort the others
    }
    Ok(())
}

/// Run the one-shot `forward` mode: a single static pair, no allow-list, no restart.
pub async fn run_forward(source: SocketAddr, target: SocketAddr, shutdown: Shutdown) -> Result<()> {
    let conns = Arc::new(AtomicUsize::new(0));
    run_forwarder(source, target, None, None, FORWARD_IDLE, conns, shutdown).await
}

/// Bind `source`, accept connections, and relay each to `target`.
///
/// `allowlist`: `Some` enforces the IP allow-list (daemon); `None` allows all (forward).
/// `restart`: if a connect to `target.port()` times out and equals this, exit(1).
pub async fn run_forwarder(
    source: SocketAddr,
    target: SocketAddr,
    allowlist: Option<Arc<Vec<String>>>,
    restart: Option<u16>,
    idle: Duration,
    conns: Arc<AtomicUsize>,
    shutdown: Shutdown,
) -> Result<()> {
    let listener = match bind_reuse(source) {
        Ok(l) => l,
        Err(e) if e.kind() == ErrorKind::AddrInUse => {
            logln!("Port {source} is already used");
            return Ok(());
        }
        Err(e) => {
            logln!("Failed to bind {source}: {e}");
            return Ok(());
        }
    };
    logln!("Forwarding {source} to {target}");
    serve_listener(listener, target, allowlist, restart, idle, conns, shutdown).await
}

/// Accept loop over an already-bound listener. Split out from [`run_forwarder`] so
/// callers (and tests) can bind an ephemeral port and learn its address first.
///
/// Equivalent to [`serve_listener_observed`] with a [`NoopObserver`] — connection
/// lifecycle events are discarded. The CLI binary uses this path, so it carries no
/// observability dependencies.
#[allow(clippy::too_many_arguments)]
pub async fn serve_listener(
    listener: TcpListener,
    target: SocketAddr,
    allowlist: Option<Arc<Vec<String>>>,
    restart: Option<u16>,
    idle: Duration,
    conns: Arc<AtomicUsize>,
    shutdown: Shutdown,
) -> Result<()> {
    serve_listener_observed(
        listener,
        target,
        allowlist,
        restart,
        idle,
        conns,
        Arc::new(NoopObserver),
        shutdown,
    )
    .await
}

/// Like [`serve_listener`], but reports each connection's lifecycle to `observer`
/// (see [`ConnEvent`]). The observer is a control-plane hook only — the byte relay in
/// [`crate::relay`] is identical either way. Embedders use this to bridge proxy
/// lifecycle into metrics, an event bus, tracing, etc. (see the `event_bus_signaling`
/// example).
#[allow(clippy::too_many_arguments)]
pub async fn serve_listener_observed(
    listener: TcpListener,
    target: SocketAddr,
    allowlist: Option<Arc<Vec<String>>>,
    restart: Option<u16>,
    idle: Duration,
    conns: Arc<AtomicUsize>,
    observer: Arc<dyn ConnObserver>,
    mut shutdown: Shutdown,
) -> Result<()> {
    let source = listener
        .local_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| "?".to_string());

    loop {
        tokio::select! {
            biased;
            _ = shutdown.wait() => {
                logln!("Proxy {source} stopped");
                return Ok(());
            }
            accepted = listener.accept() => {
                match accepted {
                    Ok((sock, peer)) => {
                        tokio::spawn(handle_connection(
                            sock,
                            peer,
                            target,
                            allowlist.clone(),
                            restart,
                            idle,
                            conns.clone(),
                            observer.clone(),
                            shutdown.clone(),
                        ));
                    }
                    // A transient accept error (e.g. fd exhaustion) must not kill the loop.
                    Err(e) => logln!("accept error on {source}: {e}"),
                }
            }
        }
    }
}

/// Bind a listener with `SO_REUSEADDR` so a quick restart doesn't hit `EADDRINUSE`
/// from a lingering `TIME_WAIT` socket.
pub fn bind_reuse(addr: SocketAddr) -> std::io::Result<TcpListener> {
    let socket = if addr.is_ipv4() {
        TcpSocket::new_v4()?
    } else {
        TcpSocket::new_v6()?
    };
    socket.set_reuseaddr(true)?;
    socket.bind(addr)?;
    socket.listen(1024)
}

#[allow(clippy::too_many_arguments)]
async fn handle_connection(
    inbound: TcpStream,
    peer: SocketAddr,
    target: SocketAddr,
    allowlist: Option<Arc<Vec<String>>>,
    restart: Option<u16>,
    idle: Duration,
    conns: Arc<AtomicUsize>,
    observer: Arc<dyn ConnObserver>,
    shutdown: Shutdown,
) {
    let remote_ip = peer.ip();

    if let Some(allow) = &allowlist {
        if !is_authorized(remote_ip, allow) {
            logln!(
                "Unknown caller {remote_ip} connection to {} rejected",
                target.port()
            );
            observer.on_event(ConnEvent::Rejected { peer, target });
            return; // drop the inbound socket
        }
    }

    let session = session_id();

    let upstream = match tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect(target)).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            if e.kind() == ErrorKind::TimedOut {
                maybe_restart(target, restart);
            }
            logln!(
                "Session {session} failed connecting to {target} - {}",
                e.kind()
            );
            observer.on_event(ConnEvent::UpstreamUnavailable {
                session,
                peer,
                target,
            });
            return;
        }
        Err(_elapsed) => {
            maybe_restart(target, restart);
            logln!("Session {session} connect to {target} timed out");
            observer.on_event(ConnEvent::UpstreamUnavailable {
                session,
                peer,
                target,
            });
            return;
        }
    };

    // Count this live connection (RAII guard decrements even on panic).
    let _guard = ConnGuard::new(&conns);
    logln!("Session {session} {remote_ip} connected to {target}");
    logln!("Total connections = {}", conns.load(Ordering::Relaxed));
    observer.on_event(ConnEvent::Opened {
        session: session.clone(),
        peer,
        target,
    });

    let stats = relay(inbound, upstream, idle, shutdown).await;

    match &stats.reason {
        ExitReason::Idle => logln!("Session {session} timeout"),
        ExitReason::Error(e) => logln!("Session {session} exception ({remote_ip}) - {}", e.kind()),
        ExitReason::Closed | ExitReason::Shutdown => {}
    }
    logln!(
        "Session {session} {remote_ip} disconnected from {target} rx {} tx {} bytes",
        group(stats.rx),
        group(stats.tx)
    );
    observer.on_event(ConnEvent::Closed {
        session,
        peer,
        target,
        rx: stats.rx,
        tx: stats.tx,
        reason: stats.reason.label(),
    });
    drop(_guard);
    logln!("Remaining connections = {}", conns.load(Ordering::Relaxed));
}

/// Returns true when a connect timeout to `target` should trigger a process restart.
/// Extracted so the condition is unit-testable without calling `process::exit`.
fn should_restart(target: SocketAddr, restart: Option<u16>) -> bool {
    restart == Some(target.port())
}

/// If the (timed-out) target equals the configured restart port, exit non-zero so a
/// process manager (systemd/docker) restarts us. Typed `TimedOut` check replaces the JS
/// locale-fragile `startsWith("connect ETIMEDOUT")`.
fn maybe_restart(target: SocketAddr, restart: Option<u16>) {
    if should_restart(target, restart) {
        logln!(
            "Stopping application because port-{} does not respond",
            target.port()
        );
        std::process::exit(1);
    }
}

/// Increments the live-connection counter on creation, decrements on drop.
struct ConnGuard<'a>(&'a Arc<AtomicUsize>);

impl<'a> ConnGuard<'a> {
    fn new(conns: &'a Arc<AtomicUsize>) -> Self {
        conns.fetch_add(1, Ordering::Relaxed);
        ConnGuard(conns)
    }
}

impl Drop for ConnGuard<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restart_triggers_on_matching_port() {
        let target: SocketAddr = "127.0.0.1:8085".parse().unwrap();
        assert!(should_restart(target, Some(8085)));
    }

    #[test]
    fn restart_does_not_trigger_on_wrong_port() {
        let target: SocketAddr = "127.0.0.1:8085".parse().unwrap();
        assert!(!should_restart(target, Some(9090)));
    }

    #[test]
    fn restart_does_not_trigger_when_unconfigured() {
        let target: SocketAddr = "127.0.0.1:8085".parse().unwrap();
        assert!(!should_restart(target, None));
    }

    #[test]
    fn conn_guard_increments_and_decrements() {
        let counter = Arc::new(AtomicUsize::new(0));
        {
            let _g1 = ConnGuard::new(&counter);
            assert_eq!(counter.load(Ordering::Relaxed), 1);
            let _g2 = ConnGuard::new(&counter);
            assert_eq!(counter.load(Ordering::Relaxed), 2);
        } // both guards dropped here
        assert_eq!(counter.load(Ordering::Relaxed), 0);
    }
}
