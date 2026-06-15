# Vision — simple-proxy

> The north star — treated as `core` (never decays) but re-confirmed on the
> invariant-verification cadence (a vision can go stale). The **Blueprint** (Open Threads
> tagged `(blueprint)` in `continuity.md`) tracks the gap from Current State to here;
> Designs and Implementations trace back to this `id`. See `DECAY.md` §12.
>
> Confirmed by the maintainer on 2026-06-14 (resolved the `(vision-bootstrap)` gate).
>
> <!-- id: vision-simple-proxy | created: 2026-06-14 | last_used: 2026-06-15 | uses: 1 | tier: core -->

## Elevator statement

A minimal, rock-solid Layer-4 TCP forwarder that doubles as the proving ground and first
consumer for a larger event-bus project — dependable enough to embed as a building block
inside a bigger system.

## Current-state context

A minimalist Layer-4 raw TCP forwarder written in Rust + Tokio, rewritten from an
original Node.js implementation in v2.0.0 (the on-disk JSON config schema was preserved
for back-compat). Protocol-agnostic NAT-style forwarding from a host port to a guest-VM
target IP/port (SSH/HTTP/HTTPS pass through as raw bytes); nginx is inspiration only —
not an HTTP/L7 proxy. Ships as a single binary with two subcommands: `serve` (config-file
daemon with optional shell-command target discovery, multiple port pairs, an `authorized`
IP allow-list, and restart-on-dead-target) and `forward` (one-shot CLI convenience, no
allow-list). The repo is a Cargo workspace: the proxy at the root plus a standalone,
reusable `event-bus` crate under `crates/` (flume-based; deliberately not in the proxy
data path — preparation for a larger event-bus project).

**Type:** CLI / utility tool (long-running daemon + one-shot CLI), single binary.

## What it should become  *(target)*

- A **production-grade, dependency-light L4 forwarder** — reliable, observable, and
  well-documented; hardened from "works on my machine" to "trust it in a real deployment."
- The **launch pad for the larger event-bus project**: `crates/event-bus` matures into a
  standalone, reusable library, with simple-proxy as its first real consumer.
- **Cleanly embeddable as a component in a larger system** — a stable public surface,
  predictable lifecycle/shutdown, and sane configuration.

## For whom

- Primarily a **building block inside a larger system/platform** (the proxy + the
  `event-bus` crate as embeddable components) — not an end-user product.

## Success criteria

- Runs **unattended in a real deployment** with auto-restart + graceful shutdown proven
  (no dropped-handler / resource-leak regressions).
- `event-bus` **stands on its own** — independently testable, documented, versioned — and
  powers **≥1 real integration beyond the demo**.
- The proxy / event-bus **embed into a larger system with no forks or patches**; the public
  API and config are stable enough to depend on.
- `cargo test / fmt / clippy` gates stay green and the minimal-dependency footprint is
  preserved.

## Non-goals

- Never a **GUI / web dashboard / admin-UI product** — stays a CLI + daemon.
- The architectural invariants already pin two hard non-goals: never an **HTTP/L7 proxy**,
  and never a **heavy dependency tree / proxy framework** (see `layer-4-only`,
  `minimal-deps` in `continuity.md`).

## Mental model

> A dumb, dependable pipe today; the seed of an event-bus platform tomorrow — minimal
> core, embeddable, never an app.
