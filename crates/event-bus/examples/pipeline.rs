//! A three-stage data pipeline wired through the event bus.
//!
//! ```
//! source → "raw" (queue) → transformer workers → "results" (broadcast) → sink
//! ```
//!
//! The "raw" route uses work-queue delivery so items are shared across two
//! transformer workers in parallel.  The "results" route uses broadcast so
//! the sink receives every processed item.
//!
//! Run with:  cargo run -p event-bus --example pipeline

use event_bus::{Event, EventBus, Receiver};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;

const N_ITEMS: usize = 8;

#[tokio::main]
async fn main() {
    let bus = Arc::new(EventBus::new());

    // Stage 2: two transformer workers share the "raw" work-queue.
    let raw_1 = bus.worker("raw");
    let raw_2 = bus.worker("raw");

    // Stage 3: sink subscribes to "results" (broadcast — it's the only subscriber).
    let results = bus.subscribe("results");

    let (done_tx, done_rx) = oneshot::channel::<Vec<String>>();

    // Spawn the downstream stages.
    let b = bus.clone();
    tokio::spawn(transformer("worker-1", raw_1, b));
    let b = bus.clone();
    tokio::spawn(transformer("worker-2", raw_2, b));
    tokio::spawn(sink(results, N_ITEMS, done_tx));

    // Stage 1: source — publish N raw items.
    for i in 0..N_ITEMS {
        let d = bus.publish_bytes("raw", format!("item-{i}").into_bytes());
        println!("source: published item-{i} -> {d:?}");
    }

    // Wait for the sink to collect all results.
    let collected = tokio::time::timeout(Duration::from_secs(5), done_rx)
        .await
        .expect("pipeline timed out")
        .unwrap();

    println!("\nSink collected {} results:", collected.len());
    for r in &collected {
        println!("  {r}");
    }
    assert_eq!(collected.len(), N_ITEMS);
}

async fn transformer(name: &'static str, rx: Receiver<Event>, bus: Arc<EventBus>) {
    while let Ok(ev) = rx.recv_async().await {
        let input = String::from_utf8_lossy(&ev.payload);
        let output = format!("[{name}] {input} processed");
        bus.publish_bytes("results", output.into_bytes());
    }
}

async fn sink(rx: Receiver<Event>, expected: usize, done: oneshot::Sender<Vec<String>>) {
    let mut results = Vec::with_capacity(expected);
    while results.len() < expected {
        match tokio::time::timeout(Duration::from_secs(5), rx.recv_async()).await {
            Ok(Ok(ev)) => results.push(String::from_utf8_lossy(&ev.payload).into_owned()),
            _ => break,
        }
    }
    let _ = done.send(results);
}
