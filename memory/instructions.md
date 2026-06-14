# Agent Instructions — simple-proxy

## What This Project Is

A small, **minimalist Layer-4 raw TCP forwarder** written in Rust + Tokio. It was
originally built to let a user SSH into a Multipass Ubuntu VM from the host machine (or
the Internet), then generalized into a socket proxy that does NAT-style forwarding from
a host port to a guest-VM target IP and port. Because it works at Layer 4, it is
protocol-agnostic (SSH/HTTP/HTTPS pass through as raw bytes). nginx is inspiration only
— it is **not** an HTTP/L7 proxy.

It was rewritten from the original Node.js implementation to Rust + Tokio in **v2.0.0**
(2026-06-13). The on-disk JSON config schema was preserved for back-compat.

**Type:** CLI / utility tool (long-running daemon + one-shot CLI), single binary.
**Primary language:** Rust (edition 2021) — built with cargo 1.95.
**Stack:** Tokio async runtime; `serde`/`serde_json` (config); `anyhow` (errors). No
HTTP/proxy framework — raw `tokio::net` sockets. The separate `event-bus` crate uses `flume`.

> High-level only. The precise dependency list and current versions live in
> `memory/continuity.md` → `## Stack & Tools` (the live source of truth).

## Repository Structure

Cargo **workspace** — the `simple-proxy` binary/lib at the repo root, plus a standalone
`event-bus` crate under `crates/`:

- `Cargo.toml` — package `simple-proxy` v2.0.0; deps tokio/serde/serde_json/anyhow.
- `src/main.rs` — thin CLI shell: dispatch, register signals once, run.
- `src/lib.rs` — re-exports the modules (so integration tests can drive them).
- `src/cli.rs` — hand-rolled arg parser → `serve` / `forward` subcommands.
- `src/config.rs` — serde config model + JSON load + validation.
- `src/discovery.rs` — async shell-command target-IP discovery + pure parser.
- `src/allowlist.rs` — exact + `x.y.z.*` IP allow-list (IPv4-mapped-IPv6 aware).
- `src/relay.rs` — the core bidirectional relay (idle-reset timeout, half-close, byte counters).
- `src/proxy.rs` — bind/accept loop, allow-list gate, upstream connect + restart, conn counter.
- `src/shutdown.rs` — `tokio::sync::watch` shutdown signal.
- `src/log.rs` — UTC-timestamped logger + INSTANCE_ID/session id + thousands grouping.
- `tests/integration.rs` — in-process echo round-trip / reject / idle / shutdown.
- `crates/event-bus/` — standalone reusable named-route event bus crate (flume; `Vec<u8>`
  payloads; broadcast + work-queue). NOT a dependency of the proxy; prep for a larger
  project. Lib at `src/lib.rs`, demo at `examples/event_bus_demo.rs`
  (`cargo run -p event-bus --example event_bus_demo`).
- `proxy-config.json` (active) + `proxy-config-multipass.json` / `-hyperv.json` (samples).
- `README.md`, `CHANGELOG.md`, `LICENSE` (Apache-2.0).

## Conventions Observed

- Idiomatic async Rust; one `select!`-loop relay over borrowing `TcpStream::split()`,
  reset-on-activity idle via `tokio::time::timeout` per read, TCP half-close on EOF.
- Logging via the `logln!` macro → `<UTC timestamp> [INSTANCE_ID] <message>`. UTC
  (not local) by design; full line built then a single `println!` for atomicity.
- Errors: `anyhow::Result` at the binary edges; per-connection failures are logged and
  contained (never abort the daemon). Typed `io::ErrorKind` matching (not error strings).
- Keep it minimal: explicit tokio feature flags (not `"full"`); no clap, no chrono.
- Run directly via `cargo run -- <subcommand>`; no codegen/build script.

## Tone & Style

- Be concise unless detail is explicitly requested.
- Prefer prose over bullet lists for explanations.
- When suggesting code changes, match the existing style and patterns in this repo.
- Always check `memory/continuity.md` for prior decisions before suggesting
  architectural changes.

## Core Rules

1. Never modify files outside the project scope without asking.
2. Follow the existing code style — do not reformat files unnecessarily.
3. When in doubt about a pattern or convention, ask rather than assume.
4. Record all significant decisions in the session log and continuity file.
5. If you see a TODO, open thread, or obvious issue, note it in continuity.md.

## Testing

`cargo test` — unit tests live in each module's `#[cfg(test)]`, plus
`tests/integration.rs` drives the real accept loop + relay against an in-process echo
server. Also gate on `cargo fmt --check` and `cargo clippy --all-targets`. Manual
end-to-end: run an upstream, `simple-proxy forward …` (or `serve`), and `curl` through it.

## CI / CD

None committed. No `.github/workflows/` or `Dockerfile`/`docker-compose.yml`, though the
README recommends systemd or docker-compose for deployment.

## Editing These Instructions

Only modify this file if the user explicitly asks to change the project
description, rules, or conventions. Treat it as stable configuration.
