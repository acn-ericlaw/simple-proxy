# event-bus

A small, general-purpose **named-route event bus** built on [`flume`](https://crates.io/crates/flume).

Two delivery models can coexist on the same route:

| Model | API | Semantics |
|-------|-----|-----------|
| **Broadcast** | `subscribe(route)` | Every subscriber gets its own copy of each event (pub/sub fan-out) |
| **Queue** | `worker(route)` | All workers share one channel — each event goes to exactly one worker |

`publish` is synchronous and never blocks (channels are unbounded).

---

## Quick Start

```rust
use event_bus::{Event, EventBus};
use std::sync::Arc;

let bus = Arc::new(EventBus::new());

// Broadcast: both subscribers receive every "metrics" event.
let sub_a = bus.subscribe("metrics");
let sub_b = bus.subscribe("metrics");

// Queue: "jobs" events are split across workers.
let worker_1 = bus.worker("jobs");
let worker_2 = bus.worker("jobs");

let d = bus.publish_bytes("metrics", b"cpu=42".to_vec());
assert_eq!(d.subscribers, 2);

let d = bus.publish(Event::new("jobs", vec![0x01]));
assert!(d.queued);
```

Consumers read via `recv_async().await` (Tokio) or `recv()` / `recv_timeout()` (sync).
Share across tasks with `Arc<EventBus>` — the bus is `Send + Sync`.

---

## API

```rust
EventBus::new() -> EventBus
EventBus::subscribe(&self, route: &str) -> Receiver<Event>
EventBus::worker(&self, route: &str) -> Receiver<Event>
EventBus::publish(&self, event: Event) -> Delivered
EventBus::publish_bytes(&self, route: &str, payload: impl Into<Vec<u8>>) -> Delivered

Delivered { subscribers: usize, queued: bool }
Delivered::total() -> usize
Delivered::is_dropped() -> bool
```

Payloads are `Vec<u8>` — text, binary, or serialised structs all pass through unchanged.

---

## Examples

```sh
# Basic pub/sub + work-queue demo
cargo run -p event-bus --example event_bus_demo

# Multi-stage processing pipeline (work-queue parallelism between stages)
cargo run -p event-bus --example pipeline
```

---

## Design Notes

- **Sync publish** — `publish` holds a short read-lock, sends to each subscriber, and
  returns. No `await` required; safe to call from sync or async code.
- **Unbounded channels** — backpressure is the caller's responsibility.
- **Dead-subscriber pruning** — `subscribe` opportunistically removes disconnected senders
  on each call; no background sweep needed.
- **No runtime dependency** — the library itself has no Tokio dependency; `recv_async` is
  available because `flume::Receiver` provides it natively. Tokio is a dev-dependency only
  (used by the examples and async unit tests).

This crate is a standalone building block. The `simple-proxy` binary does not depend on it —
it is designed to be embedded into a larger system as an event backbone.
