//! A small, general-purpose **named-route event bus** built on `flume`.
//!
//! Routes are addressed by string name. Two delivery models are supported, and they
//! may coexist on the same route:
//!
//! * **broadcast** ([`EventBus::subscribe`]) — every subscriber on a route receives its
//!   own copy of each published event (pub/sub fan-out).
//! * **queue** ([`EventBus::worker`]) — workers on a route share one channel, so each
//!   event is delivered to exactly one worker (work-stealing / load-sharing).
//!
//! [`EventBus::publish`] is synchronous and never blocks (channels are unbounded).
//! Consumers read through a [`Receiver`]: `recv_async().await` inside Tokio tasks, or
//! `recv()` / `recv_timeout()` synchronously.
//!
//! The bus is `Send + Sync`; share it across tasks as `Arc<EventBus>`.
//!
//! This is a standalone building-block crate — the `simple-proxy` binary does not depend
//! on it — created as preparation for a larger event-bus project.

use std::collections::HashMap;
use std::sync::RwLock;

pub use flume::Receiver;
use flume::Sender;

/// A routed message. `payload` is an opaque byte array (binary-safe).
#[derive(Debug, Clone)]
pub struct Event {
    pub route: String,
    pub payload: Vec<u8>,
}

impl Event {
    pub fn new(route: impl Into<String>, payload: impl Into<Vec<u8>>) -> Self {
        Self {
            route: route.into(),
            payload: payload.into(),
        }
    }
}

/// Outcome of a [`EventBus::publish`]: how many consumers the event reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Delivered {
    /// Number of broadcast subscribers the event was copied to.
    pub subscribers: usize,
    /// Whether the event was enqueued for a work-queue on the route.
    pub queued: bool,
}

impl Delivered {
    /// Total number of consumers the event reached.
    pub fn total(&self) -> usize {
        self.subscribers + usize::from(self.queued)
    }

    /// True if the event reached no consumer (no matching route).
    pub fn is_dropped(&self) -> bool {
        self.total() == 0
    }
}

/// The shared (sender, receiver) pair backing a route's work-queue.
type QueueChannel = (Sender<Event>, Receiver<Event>);

/// A named-route event bus supporting both broadcast (pub/sub) and work-queue delivery.
#[derive(Default)]
pub struct EventBus {
    /// route -> one sender per broadcast subscriber.
    broadcast: RwLock<HashMap<String, Vec<Sender<Event>>>>,
    /// route -> the shared work-queue channel (created lazily on first `worker`).
    queues: RwLock<HashMap<String, QueueChannel>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a broadcast subscriber on `route`. Each call returns an independent
    /// receiver; every published event is copied to all current subscribers.
    pub fn subscribe(&self, route: &str) -> Receiver<Event> {
        let (tx, rx) = flume::unbounded();
        let mut map = self.broadcast.write().unwrap();
        let subs = map.entry(route.to_string()).or_default();
        subs.retain(|s| !s.is_disconnected()); // opportunistically drop dead subscribers
        subs.push(tx);
        rx
    }

    /// Register a work-queue consumer on `route`. All workers for a route share one
    /// channel, so each published event is delivered to exactly one of them.
    pub fn worker(&self, route: &str) -> Receiver<Event> {
        let mut map = self.queues.write().unwrap();
        let (_, rx) = map
            .entry(route.to_string())
            .or_insert_with(flume::unbounded);
        rx.clone()
    }

    /// Publish an event to its route. Synchronous and non-blocking; returns how many
    /// consumers it reached. Delivers a copy to every broadcast subscriber and one copy
    /// to the route's work-queue (if either exists).
    pub fn publish(&self, event: Event) -> Delivered {
        let mut subscribers = 0;
        {
            let map = self.broadcast.read().unwrap();
            if let Some(subs) = map.get(&event.route) {
                for s in subs {
                    // A send only fails if every receiver for that subscriber is gone.
                    if s.send(event.clone()).is_ok() {
                        subscribers += 1;
                    }
                }
            }
        }

        let queued = {
            let map = self.queues.read().unwrap();
            match map.get(&event.route) {
                Some((tx, _)) => tx.send(event).is_ok(),
                None => false,
            }
        };

        Delivered {
            subscribers,
            queued,
        }
    }

    /// Convenience: publish raw bytes to `route`.
    pub fn publish_bytes(&self, route: &str, payload: impl Into<Vec<u8>>) -> Delivered {
        self.publish(Event::new(route, payload))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;

    fn recv(rx: &Receiver<Event>) -> Option<Event> {
        rx.recv_timeout(Duration::from_millis(500)).ok()
    }

    #[test]
    fn broadcast_fans_out_to_all_subscribers() {
        let bus = EventBus::new();
        let a = bus.subscribe("evt");
        let b = bus.subscribe("evt");

        let d = bus.publish_bytes("evt", b"hello".to_vec());
        assert_eq!(d.subscribers, 2);
        assert!(!d.queued);

        assert_eq!(recv(&a).unwrap().payload, b"hello");
        assert_eq!(recv(&b).unwrap().payload, b"hello");
    }

    #[test]
    fn queue_delivers_each_event_exactly_once() {
        let bus = EventBus::new();
        let w1 = bus.worker("jobs");
        let w2 = bus.worker("jobs");

        for i in 0..4u8 {
            let d = bus.publish(Event::new("jobs", vec![i]));
            assert!(d.queued);
            assert_eq!(d.subscribers, 0);
        }

        // The two workers share one queue; gather everything they receive.
        let mut got = Vec::new();
        while let Some(e) = recv(&w1) {
            got.push(e.payload[0]);
        }
        while let Some(e) = recv(&w2) {
            got.push(e.payload[0]);
        }
        got.sort_unstable();
        assert_eq!(got, vec![0, 1, 2, 3], "each event delivered once, no dupes");
    }

    #[test]
    fn hybrid_route_reaches_subscriber_and_worker() {
        let bus = EventBus::new();
        let sub = bus.subscribe("mix");
        let work = bus.worker("mix");

        let d = bus.publish_bytes("mix", b"x".to_vec());
        assert_eq!(d.subscribers, 1);
        assert!(d.queued);
        assert!(recv(&sub).is_some());
        assert!(recv(&work).is_some());
    }

    #[test]
    fn unknown_route_is_dropped() {
        let bus = EventBus::new();
        let d = bus.publish_bytes("nope", b"x".to_vec());
        assert!(d.is_dropped());
        assert_eq!(d.total(), 0);
    }

    #[test]
    fn binary_payload_roundtrips() {
        let bus = EventBus::new();
        let rx = bus.subscribe("bin");
        let bytes = vec![0u8, 1, 2, 255, 254, 0];
        bus.publish(Event::new("bin", bytes.clone()));
        assert_eq!(recv(&rx).unwrap().payload, bytes);
    }

    #[test]
    fn dropped_subscriber_is_pruned() {
        let bus = EventBus::new();
        let a = bus.subscribe("r");
        drop(a); // disconnects a's channel
        let _b = bus.subscribe("r"); // prunes the dead 'a' on the way in

        let d = bus.publish_bytes("r", b"x".to_vec());
        assert_eq!(d.subscribers, 1, "only the live subscriber is counted");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn shareable_across_tasks() {
        let bus = Arc::new(EventBus::new());
        let rx = bus.subscribe("t");

        let producer = bus.clone();
        tokio::spawn(async move {
            producer.publish_bytes("t", b"async".to_vec());
        })
        .await
        .unwrap();

        assert_eq!(rx.recv_async().await.unwrap().payload, b"async");
    }
}
