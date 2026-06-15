# Vision — event-bus (in-process event backbone)

> The north star — treated as `core` (never decays) but re-confirmed on the
> invariant-verification cadence (a vision can go stale). The **Blueprint** (Open Threads
> tagged `(blueprint)` in `continuity.md`) tracks the gap from Current State to here;
> Designs and Implementations trace back to this `id`. See `DECAY.md` §12.
>
> Confirmed by the maintainer on 2026-06-15 — promoting the `event-bus` crate to the new
> Vision after the prior Vision (`vision-simple-proxy`) was realized. Resolved the
> `ot-vision-gap-closed-rehorizon` human gate.
>
> <!-- id: vision-event-bus | created: 2026-06-15 | last_used: 2026-06-15 | uses: 1 | tier: core | supersedes: vision-simple-proxy -->

## Elevator statement

A minimal, embeddable **in-process event backbone** for Rust systems — bounded,
observable, and typed-optional — that larger applications depend on as their internal
nervous system without forks or patches.

## Current-state context

`crates/event-bus` (v0.1.0) is a standalone named-route event bus built on `flume`. Routes
are addressed by string name; two delivery models coexist on a route — **broadcast**
(`subscribe`, every subscriber gets a copy) and **work-queue** (`worker`, each event to
exactly one worker). Payloads are opaque `Vec<u8>` (binary-safe). `publish` is synchronous
and non-blocking; the bus is `Send + Sync` and shared as `Arc<EventBus>`. The library has
no runtime dependency (flume provides `recv_async`); Tokio is dev-only.

Known limits at the starting line: channels are **unbounded** (backpressure explicitly
punted to the caller), there is **no typed payload layer** (callers serialise by hand), **no
observability** (queue depth / drop / consumer counts), **no route lifecycle** (the broadcast
and queue maps grow without teardown; only dead broadcast subscribers are pruned), and the
only "consumers" are two examples (`event_bus_demo`, `pipeline`). The crate is `0.1.0` —
not yet semver-stable or published.

`simple-proxy` — the realized predecessor Vision — stays **independent** and in maintenance
for its core L4 forwarding. It is, however, the planned **demonstration / experiment** for
the bus: wire the event-bus into the proxy's **control plane** (connection-lifecycle signals
— socket open / close, etc.), *never* the byte data path (the relay stays direct, per the
`layer-4-only` invariant and the direct-relay decision). Because the shipping `simple-proxy`
binary must link no flume (`minimal-deps` core invariant), this demo lives as a **separate
example / feature-gated artifact**, not in the default binary. The experiment is a *learning
vehicle* — refine the bus from what it teaches us — and is distinct from the success
criterion's **real-world consumer**, which must be something other than simple-proxy.

**Type:** Embeddable Rust library crate (in-process building block) — not a binary, daemon,
or service.

## What it should become  *(target)*

- A **production-grade in-process event backbone**: bounded delivery with a defined overflow
  policy, observable, and embeddable into larger Rust systems without forks.
- **Typed-optional**: a typed/serde convenience layer over the `Vec<u8>` core, available
  without forcing a serde dependency on byte-level users.
- **Semver-stable and published** (crates.io or an internal registry), with a documented,
  `#[non_exhaustive]`-where-appropriate public API a larger system can depend on.
- Proven by **≥1 real-world consumer** (other than simple-proxy and other than the
  examples/demos), reached via the proving path below.

## For whom

- Primarily an **embeddable building block inside a larger Rust system/platform** — the
  internal event/message fabric of an application — not an end-user product and not a
  network service.

## Success criteria

- **Bounded delivery**: configurable bounded channels with an explicit overflow policy
  (block / drop-oldest / reject), replacing the unconditional unbounded channels.
- **Typed-optional API**: publish/subscribe typed values via an opt-in layer without
  forcing serde on `Vec<u8>` users; minimal-deps footprint preserved (feature-gated).
- **Observable**: per-route metrics (queue depth, delivered / dropped counts, subscriber /
  worker counts) exposed through a stable API.
- **Route lifecycle**: routes/queues can be torn down or reclaimed so the internal maps
  don't grow unbounded.
- **Stable & versioned**: a semver-stable, documented, published crate.
- **≥1 real-world integration** — a genuine consumer that is *not* simple-proxy and *not* an
  example/demo. (The simple-proxy control-plane signaling demo is the *learning experiment*
  that precedes and informs this — not the criterion itself.)
- `cargo test / fmt / clippy` gates stay green; the minimal-dependency footprint is preserved.

## Non-goals

- **Never networked / distributed** — stays an in-process library. No wire protocol, no
  transport, no broker daemon, no cross-host delivery. (If networking is ever wanted, it is
  a *future* Vision, opened through a fresh human gate — not this one.)
- **Never an app** — no GUI, web dashboard, CLI front-end, or standalone service.
- **Never a heavy dependency tree** — keep the minimal-deps ethos inherited from
  simple-proxy: flume at the core, optional/feature-gated extras only.

## Proving path

1. **Experiment** — wire event-bus **control-plane signaling** (connection open / close
   lifecycle) into a simple-proxy *demo* (a separate example / feature-gated artifact; the
   shipping proxy binary stays flume-free; data path stays direct).
2. **Learn & refine** — improve the bus from what the experiment exposes (backpressure
   behaviour, observability, route lifecycle, typed ergonomics) *before* committing to a
   real-world use case.
3. **Apply** — adopt the refined bus in a **real-world consumer** (not simple-proxy, not a
   demo) — the success criterion.

## Mental model

> A dependable in-process nervous system for a Rust application — minimal, embeddable,
> bounded, and observable; never a network broker, never an app.

## Lineage

The prior Vision (`vision-simple-proxy`) — *"a minimal, rock-solid L4 TCP forwarder that
doubles as the proving ground and first consumer for a larger event-bus project"* — was
**realized** on 2026-06-15: all five of its Blueprint gaps closed (committed & pushed,
CI green, 48 tests, embeddable surface, matured event-bus crate). Per the maintainer's
2026-06-15 decision it is superseded — *as realized, not as wrong* — by this Vision, which
carries the event-bus forward as the new north star. The realized record is archived
(`memory/archive/2026-Q2.md`, flagged `superseded-by: vision-event-bus`).
