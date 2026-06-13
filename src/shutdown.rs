//! A tiny shutdown-signal primitive built on `tokio::sync::watch` (no extra crate).
//! One controller flips the flag; any number of cloned [`Shutdown`] handles observe it.

use tokio::sync::watch;

/// Held by `main`'s signal task; flips every observer to "shutting down".
pub struct ShutdownController {
    tx: watch::Sender<bool>,
}

/// A cloneable observer. Each task (accept loop, relay) holds its own.
#[derive(Clone)]
pub struct Shutdown {
    rx: watch::Receiver<bool>,
}

/// Create a linked `(controller, observer)` pair.
pub fn channel() -> (ShutdownController, Shutdown) {
    let (tx, rx) = watch::channel(false);
    (ShutdownController { tx }, Shutdown { rx })
}

impl ShutdownController {
    /// Signal shutdown to all observers. Idempotent.
    pub fn trigger(&self) {
        let _ = self.tx.send(true);
    }
}

impl Shutdown {
    /// Resolves as soon as shutdown has been signalled (returns immediately if it
    /// already has). Also returns if the controller was dropped.
    pub async fn wait(&mut self) {
        loop {
            if *self.rx.borrow_and_update() {
                return;
            }
            if self.rx.changed().await.is_err() {
                return; // controller dropped — treat as shutdown
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn wait_returns_after_trigger() {
        let (ctrl, mut sd) = channel();
        ctrl.trigger();
        // Should resolve essentially immediately.
        tokio::time::timeout(std::time::Duration::from_secs(1), sd.wait())
            .await
            .expect("wait did not resolve after trigger");
    }

    #[tokio::test]
    async fn wait_resolves_when_triggered_later() {
        let (ctrl, mut sd) = channel();
        let h = tokio::spawn(async move { sd.wait().await });
        tokio::task::yield_now().await;
        ctrl.trigger();
        tokio::time::timeout(std::time::Duration::from_secs(1), h)
            .await
            .expect("timed out")
            .expect("join failed");
    }
}
