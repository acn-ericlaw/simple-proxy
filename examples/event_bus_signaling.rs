//! Worked example: bridge simple-proxy's **control-plane** lifecycle onto the
//! `event-bus` crate, while the proxy keeps doing its real Layer-4 forwarding.
//!
//! ```text
//!   accepted / opened / closed / rejected
//!         │   (impl ConnObserver)
//!         ▼
//!     EventBus ──┬─ "conn.events" (broadcast) ─► monitor : prints every event live
//!                └─ "conn.closed" (work-queue) ─► metrics: tallies conns + bytes
//! ```
//!
//! Key points this demonstrates:
//! * The proxy's byte path ([`relay`](simple_proxy::relay)) is untouched — the bus only
//!   carries control-plane signals emitted *around* the relay.
//! * Both event-bus delivery models at once: **broadcast** (every subscriber sees every
//!   event) and **work-queue** (each Closed event is handled by exactly one worker).
//! * `event-bus` is a dev-dependency, so it is NOT linked into the `simple-proxy` binary.
//!
//! Run:  `cargo run --example event_bus_signaling`
//!
//! Output interleaves the proxy's own `logln!` lines (tagged `[NNNN]`) with the demo's
//! `[monitor]` / summary lines — that's the proxy and the bus working side by side.

use std::net::SocketAddr;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::time::Duration;

use event_bus::{Event, EventBus, Receiver};
use simple_proxy::observer::{ConnEvent, ConnObserver};
use simple_proxy::proxy::{bind_reuse, serve_listener_observed};
use simple_proxy::shutdown;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

/// Number of successful client round-trips to drive through the proxy.
const ROUND_TRIPS: usize = 3;

/// A [`ConnObserver`] that republishes each proxy lifecycle event onto the event bus.
struct BusObserver {
    bus: Arc<EventBus>,
}

impl ConnObserver for BusObserver {
    fn on_event(&self, event: ConnEvent) {
        // Fan every event out to broadcast subscribers (the live monitor).
        self.bus
            .publish_bytes("conn.events", describe(&event).into_bytes());
        // Closed events also enqueue a compact record for the metrics work-queue.
        if let ConnEvent::Closed { rx, tx, .. } = &event {
            self.bus
                .publish_bytes("conn.closed", format!("{rx} {tx}").into_bytes());
        }
    }
}

/// One human-readable line describing an event (consumed by the monitor).
fn describe(event: &ConnEvent) -> String {
    match event {
        ConnEvent::Opened {
            session,
            peer,
            target,
        } => format!("OPENED      session={session} {peer} -> {target}"),
        ConnEvent::Closed {
            session,
            peer,
            target,
            rx,
            tx,
            reason,
        } => format!("CLOSED      session={session} {peer} -> {target} rx={rx} tx={tx} ({reason})"),
        ConnEvent::Rejected { peer, target } => {
            format!("REJECTED    {peer} -> {target} (allow-list)")
        }
        ConnEvent::UpstreamUnavailable {
            session,
            peer,
            target,
        } => format!("NO-UPSTREAM session={session} {peer} -> {target}"),
        // `ConnEvent` is #[non_exhaustive]; tolerate variants added in future versions.
        _ => "UNKNOWN     (unrecognized event)".to_string(),
    }
}

#[tokio::main]
async fn main() {
    let bus = Arc::new(EventBus::new());

    // ── Consumers ──────────────────────────────────────────────────────────────
    // Broadcast: a live monitor prints every lifecycle event.
    tokio::spawn(monitor(bus.subscribe("conn.events")));

    // Work-queue: a metrics aggregator drains Closed events and tallies totals.
    // (Spawn more `bus.worker("conn.closed")` consumers and they would load-share it.)
    let (summary_tx, summary_rx) = oneshot::channel();
    tokio::spawn(metrics(bus.worker("conn.closed"), ROUND_TRIPS, summary_tx));

    let observer: Arc<dyn ConnObserver> = Arc::new(BusObserver { bus });

    // ── The real proxy, forwarding to an in-process echo upstream ───────────────
    let echo = spawn_echo().await;
    let proxy_addr = spawn_proxy(echo, vec!["127.0.0.1".into()], observer.clone()).await;
    // A second, locked-down proxy whose allow-list excludes loopback — to show Rejected.
    let locked_addr = spawn_proxy(echo, vec!["10.0.0.1".into()], observer.clone()).await;

    println!("proxy {proxy_addr} (open) / {locked_addr} (locked) -> echo {echo}\n");

    // ── Drive real traffic: ROUND_TRIPS client round-trips through the open proxy ─
    for i in 0..ROUND_TRIPS {
        let mut client = TcpStream::connect(proxy_addr).await.unwrap();
        let msg = format!("ping-{i}");
        client.write_all(msg.as_bytes()).await.unwrap();
        let mut buf = vec![0u8; msg.len()];
        client.read_exact(&mut buf).await.unwrap();
        assert_eq!(
            buf.as_slice(),
            msg.as_bytes(),
            "echo must round-trip intact"
        );
        drop(client); // close → relay tears down → Closed event
    }

    // One connection the locked proxy will reject (dropped before any relay).
    if let Ok(mut client) = TcpStream::connect(locked_addr).await {
        let _ = client.write_all(b"nope").await;
        let mut buf = [0u8; 8];
        let _ = client.read(&mut buf).await; // returns EOF: rejected before relay
    }

    // ── Wait for the work-queue worker to see every Closed event, then report ────
    let summary = tokio::time::timeout(Duration::from_secs(5), summary_rx)
        .await
        .expect("metrics worker timed out")
        .expect("metrics worker dropped");

    // Brief grace so the monitor flushes its last lines before the summary block.
    tokio::time::sleep(Duration::from_millis(50)).await;

    println!("\n── metrics (aggregated off the \"conn.closed\" work-queue) ──");
    println!("connections closed : {}", summary.count);
    println!("bytes client->up   : {}", summary.rx);
    println!("bytes up->client   : {}", summary.tx);
}

/// Aggregated totals reported by the metrics worker.
struct Summary {
    count: usize,
    rx: u64,
    tx: u64,
}

/// Broadcast consumer: print every lifecycle event as it arrives.
async fn monitor(rx: Receiver<Event>) {
    while let Ok(ev) = rx.recv_async().await {
        println!("[monitor] {}", String::from_utf8_lossy(&ev.payload));
    }
}

/// Work-queue consumer: accumulate `expected` Closed records, then send the totals.
async fn metrics(rx: Receiver<Event>, expected: usize, done: oneshot::Sender<Summary>) {
    let mut s = Summary {
        count: 0,
        rx: 0,
        tx: 0,
    };
    while s.count < expected {
        match tokio::time::timeout(Duration::from_secs(5), rx.recv_async()).await {
            Ok(Ok(ev)) => {
                let text = String::from_utf8_lossy(&ev.payload);
                let mut fields = text.split_whitespace();
                s.rx += fields.next().and_then(|n| n.parse().ok()).unwrap_or(0);
                s.tx += fields.next().and_then(|n| n.parse().ok()).unwrap_or(0);
                s.count += 1;
            }
            _ => break, // timed out or the bus closed
        }
    }
    let _ = done.send(s);
}

/// Start the real proxy on an ephemeral loopback port with the given allow-list and
/// observer; returns its bound address. Mirrors how the integration tests drive it.
async fn spawn_proxy(
    target: SocketAddr,
    allow: Vec<String>,
    observer: Arc<dyn ConnObserver>,
) -> SocketAddr {
    let listener = bind_reuse("127.0.0.1:0".parse().unwrap()).unwrap();
    let addr = listener.local_addr().unwrap();
    let (ctrl, sd) = shutdown::channel();
    let conns = Arc::new(AtomicUsize::new(0));
    let allow = Some(Arc::new(allow));
    tokio::spawn(async move {
        // Keep the controller alive for the task's life: dropping it would signal
        // shutdown (see `Shutdown::wait`). The task is aborted when `main` returns.
        let _ctrl = ctrl;
        let _ = serve_listener_observed(
            listener,
            target,
            allow,
            None,
            Duration::from_secs(30),
            conns,
            observer,
            sd,
        )
        .await;
    });
    addr
}

/// An in-process echo server on an ephemeral loopback port; returns its address.
async fn spawn_echo() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((mut sock, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                loop {
                    match sock.read(&mut buf).await {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            if sock.write_all(&buf[..n]).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            });
        }
    });
    addr
}
