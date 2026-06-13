//! End-to-end tests: drive the real accept loop + relay in-process against an
//! in-process echo server.

use simple_proxy::proxy::{bind_reuse, serve_listener};
use simple_proxy::shutdown;
use std::net::SocketAddr;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Spawn an echo server on an ephemeral loopback port; returns its address.
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

/// Start a forwarder on an ephemeral loopback port; returns its address + a shutdown handle.
async fn spawn_proxy(
    target: SocketAddr,
    allowlist: Option<Vec<String>>,
    idle: Duration,
) -> (
    SocketAddr,
    shutdown::ShutdownController,
    tokio::task::JoinHandle<()>,
) {
    let listener = bind_reuse("127.0.0.1:0".parse().unwrap()).unwrap();
    let source = listener.local_addr().unwrap();
    let (ctrl, sd) = shutdown::channel();
    let allow = allowlist.map(Arc::new);
    let conns = Arc::new(AtomicUsize::new(0));
    let handle = tokio::spawn(async move {
        let _ = serve_listener(listener, target, allow, None, idle, conns, sd).await;
    });
    (source, ctrl, handle)
}

#[tokio::test]
async fn bytes_round_trip_through_proxy() {
    let echo = spawn_echo().await;
    let (proxy, _ctrl, _h) = spawn_proxy(
        echo,
        Some(vec!["127.0.0.1".into()]),
        Duration::from_secs(30),
    )
    .await;

    let mut client = TcpStream::connect(proxy).await.unwrap();
    let payload = b"hello layer-4 world";
    client.write_all(payload).await.unwrap();

    let mut buf = vec![0u8; payload.len()];
    client.read_exact(&mut buf).await.unwrap();
    assert_eq!(
        &buf, payload,
        "payload must echo back unchanged through the proxy"
    );
}

#[tokio::test]
async fn larger_payload_round_trips() {
    let echo = spawn_echo().await;
    let (proxy, _ctrl, _h) = spawn_proxy(
        echo,
        Some(vec!["127.0.0.1".into()]),
        Duration::from_secs(30),
    )
    .await;

    let client = TcpStream::connect(proxy).await.unwrap();
    let (mut r, mut w) = client.into_split();
    let payload: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();

    // Read concurrently with writing to avoid backpressure deadlock on a 200KB echo.
    let p = payload.clone();
    let writer = tokio::spawn(async move {
        w.write_all(&p).await.unwrap();
        w.shutdown().await.unwrap(); // half-close so the echo + relay drain and EOF
    });

    let mut got = Vec::new();
    r.read_to_end(&mut got).await.unwrap();
    writer.await.unwrap();
    assert_eq!(got, payload, "all {} bytes must round-trip", payload.len());
}

#[tokio::test]
async fn unauthorized_client_is_rejected() {
    let echo = spawn_echo().await;
    // Allow-list excludes loopback, so our 127.0.0.1 client must be dropped.
    let (proxy, _ctrl, _h) =
        spawn_proxy(echo, Some(vec!["10.0.0.1".into()]), Duration::from_secs(30)).await;

    let mut client = TcpStream::connect(proxy).await.unwrap();
    client.write_all(b"should be dropped").await.ok();

    // The proxy rejects before relaying, so the read returns EOF (0) with no echo.
    let mut buf = [0u8; 16];
    let n = tokio::time::timeout(Duration::from_secs(2), client.read(&mut buf))
        .await
        .expect("read should not hang")
        .unwrap_or(0);
    assert_eq!(n, 0, "rejected connection must not echo any bytes");
}

#[tokio::test]
async fn idle_connection_times_out() {
    let echo = spawn_echo().await;
    // 200ms idle window: send nothing and the relay should close the connection.
    let (proxy, _ctrl, _h) = spawn_proxy(
        echo,
        Some(vec!["127.0.0.1".into()]),
        Duration::from_millis(200),
    )
    .await;

    let mut client = TcpStream::connect(proxy).await.unwrap();
    // Stay silent; expect the proxy to close us within a couple of idle windows.
    let mut buf = [0u8; 16];
    let n = tokio::time::timeout(Duration::from_secs(2), client.read(&mut buf))
        .await
        .expect("idle timeout should close the connection")
        .unwrap_or(0);
    assert_eq!(n, 0, "idle connection should be closed (EOF)");
}

#[tokio::test]
async fn graceful_shutdown_stops_accept_loop() {
    let echo = spawn_echo().await;
    let (_proxy, ctrl, handle) = spawn_proxy(
        echo,
        Some(vec!["127.0.0.1".into()]),
        Duration::from_secs(30),
    )
    .await;

    ctrl.trigger();
    tokio::time::timeout(Duration::from_secs(2), handle)
        .await
        .expect("accept loop should stop after shutdown")
        .expect("accept loop task panicked");
}
