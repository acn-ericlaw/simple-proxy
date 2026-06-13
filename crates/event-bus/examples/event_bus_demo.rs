//! Demonstrates the named-route event bus in both delivery modes, with byte payloads.
//!
//! Run with:  cargo run -p event-bus --example event_bus_demo

use event_bus::{Event, EventBus, Receiver};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let bus = Arc::new(EventBus::new());

    // Broadcast (pub/sub): both subscribers receive a copy of every "metrics" event.
    let sub_a = bus.subscribe("metrics");
    let sub_b = bus.subscribe("metrics");

    // Queue (work-sharing): "jobs" events are split across the two workers.
    let worker_1 = bus.worker("jobs");
    let worker_2 = bus.worker("jobs");

    tokio::spawn(consume("sub-A   ", sub_a));
    tokio::spawn(consume("sub-B   ", sub_b));
    tokio::spawn(consume("worker-1", worker_1));
    tokio::spawn(consume("worker-2", worker_2));

    // Text and raw-binary payloads both work (payload is Vec<u8>).
    println!(
        "publish metrics cpu=42       -> {:?}",
        bus.publish_bytes("metrics", b"cpu=42".to_vec())
    );
    println!(
        "publish metrics [00 01 FF]   -> {:?}",
        bus.publish_bytes("metrics", vec![0x00, 0x01, 0xFF])
    );

    for i in 0..4u8 {
        let d = bus.publish(Event::new("jobs", vec![i]));
        println!("publish jobs    [{i}]          -> {d:?}");
    }

    // No subscriber/worker on this route -> dropped.
    println!(
        "publish unknown              -> {:?}",
        bus.publish_bytes("unknown", b"nobody home".to_vec())
    );

    // Let the async consumers print before we exit.
    tokio::time::sleep(Duration::from_millis(200)).await;
}

async fn consume(name: &str, rx: Receiver<Event>) {
    while let Ok(ev) = rx.recv_async().await {
        println!("  [{name}] route={:<8} payload={:?}", ev.route, ev.payload);
    }
}
