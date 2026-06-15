//! The bidirectional byte relay — the heart of the L4 forwarder.
//!
//! `tokio::io::copy_bidirectional` would give byte counts but cannot reset an idle
//! timer on activity, so we hand-roll a single `select!` loop over the split halves
//! and wrap each read in `tokio::time::timeout(idle, ..)`. A fresh timeout per
//! iteration yields reset-on-activity semantics (matching the JS `socket.setTimeout`).
//! Either side closing (EOF) triggers immediate full teardown of the relay so that
//! keep-alive servers don't hold connections open after the client has gone.

use crate::shutdown::Shutdown;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const BUF_SIZE: usize = 16 * 1024;

/// Why a relay ended. `rx`/`tx` are always reported regardless of reason.
pub enum ExitReason {
    /// Both directions reached EOF (normal close).
    Closed,
    /// No activity in either direction within the idle window.
    Idle,
    /// A graceful shutdown was requested.
    Shutdown,
    /// An I/O error on one of the sockets.
    Error(std::io::Error),
}

pub struct RelayStats {
    /// Bytes client -> upstream (the inbound socket's `bytesRead`).
    pub rx: u64,
    /// Bytes upstream -> client (the inbound socket's `bytesWritten`).
    pub tx: u64,
    pub reason: ExitReason,
}

/// Relay bytes between `inbound` (the accepted client) and `upstream` (the target)
/// until both close, the connection goes idle, an error occurs, or shutdown fires.
pub async fn relay(
    mut inbound: TcpStream,
    mut upstream: TcpStream,
    idle: Duration,
    mut shutdown: Shutdown,
) -> RelayStats {
    let (mut ri, mut wi) = inbound.split();
    let (mut ru, mut wu) = upstream.split();
    let mut rx: u64 = 0;
    let mut tx: u64 = 0;
    let mut buf_c = vec![0u8; BUF_SIZE];
    let mut buf_u = vec![0u8; BUF_SIZE];

    let reason = loop {
        tokio::select! {
            biased;

            _ = shutdown.wait() => {
                let _ = wu.shutdown().await;
                let _ = wi.shutdown().await;
                break ExitReason::Shutdown;
            }

            // client -> upstream: either side closing tears down the whole relay.
            r = tokio::time::timeout(idle, ri.read(&mut buf_c)) => match r {
                Err(_elapsed) => break ExitReason::Idle,
                Ok(Ok(0)) => {
                    let _ = wu.shutdown().await;
                    let _ = wi.shutdown().await;
                    break ExitReason::Closed;
                }
                Ok(Ok(n)) => match wu.write_all(&buf_c[..n]).await {
                    Ok(()) => rx += n as u64,
                    Err(e) => break ExitReason::Error(e),
                },
                Ok(Err(e)) => break ExitReason::Error(e),
            },

            // upstream -> client: same symmetric teardown.
            r = tokio::time::timeout(idle, ru.read(&mut buf_u)) => match r {
                Err(_elapsed) => break ExitReason::Idle,
                Ok(Ok(0)) => {
                    let _ = wi.shutdown().await;
                    break ExitReason::Closed;
                }
                Ok(Ok(n)) => match wi.write_all(&buf_u[..n]).await {
                    Ok(()) => tx += n as u64,
                    Err(e) => break ExitReason::Error(e),
                },
                Ok(Err(e)) => break ExitReason::Error(e),
            },
        }
    };

    RelayStats { rx, tx, reason }
}
