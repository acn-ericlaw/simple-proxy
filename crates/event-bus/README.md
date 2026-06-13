# event-bus

A small, general-purpose **named-route event bus** for async Rust, built on `flume`.

- Routes addressed by string name; `Vec<u8>` (binary-safe) payloads.
- Two delivery models, which may coexist on the same route:
  - **broadcast** (`subscribe`) — every subscriber gets its own copy (pub/sub fan-out).
  - **work-queue** (`worker`) — workers on a route share one channel; each event goes to exactly one.
- `publish` is synchronous and non-blocking; returns how many consumers it reached.
- `Send + Sync` — share as `Arc<EventBus>` across tasks.

```rust
use event_bus::{Event, EventBus};

let bus = EventBus::new();
let rx = bus.subscribe("metrics");
bus.publish_bytes("metrics", b"cpu=42".to_vec());
let event = rx.recv().unwrap(); // or rx.recv_async().await in a Tokio task
```

Run the demo:

```sh
cargo run -p event-bus --example event_bus_demo
```
