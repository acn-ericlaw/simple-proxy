# Changelog

## Release notes

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> This changelog was introduced when the simple-proxy source was imported into this
> repo at v1.2.0. The granular 1.0.0 → 1.2.0 history predates this repo and is not
> reconstructed; the 1.2.0 entry below describes the feature set as imported,
> organized by capability rather than by individual commit.

---
## Version 2.0.0, 6/13/2026

> **Breaking: full rewrite from Node.js to Rust + Tokio.** The tool keeps its identity
> — a lightweight Layer-4 raw TCP forwarder that carries SSH/HTTP/HTTPS as opaque bytes
> — but ships as a single statically-typed async binary instead of two `.js` files run
> under Node. The on-disk JSON config schema is preserved, so existing `proxy-config.json`
> files keep working. nginx is inspiration only; this is not an HTTP/L7 proxy.

### Added

1. Rust + Tokio implementation: one binary `simple-proxy` with two subcommands —
   `serve [--config <path>]` (config-driven daemon) and `forward <src> <dst>` (one-shot
   pair). Modules: `cli`, `config`, `discovery`, `allowlist`, `relay`, `proxy`,
   `shutdown`, `log`.
2. Async bidirectional relay with a reset-on-activity idle timeout and proper TCP
   half-close (closes only the finished direction, draining the other).
3. Optional discovery: omit the `discovery` block and set a static `target_ip` instead
   — a simpler config path for non-VM targets, while preserving shell-command IP
   discovery for the Multipass/Hyper-V use case.
4. `SO_REUSEADDR` on listeners (clean restart under a process manager / docker), and a bounded
   upstream-connect timeout.
5. Unit + integration test suite (`cargo test`): allow-list matching incl. IPv4-mapped
   IPv6, config parsing/validation, discovery output parsing, CLI parsing incl. IPv6,
   and in-process echo round-trip / allow-list reject / idle / graceful-shutdown tests.
6. `CHANGELOG.md` (this file) and AI-enablement via the **agent-memory** shared memory
   system (Mode A: `AGENTS.md`, per-vendor bootstrap pointers, the `memory/` layer, and
   the `.agent/` manifest).
7. The repo is now a Cargo **workspace**: the proxy at the root plus a standalone
   `event-bus` crate (`crates/event-bus/`) — a general-purpose named-route event bus
   (flume; `Vec<u8>` payloads; broadcast + work-queue) built as preparation for a larger
   project. The proxy binary does not depend on it.

### Removed

1. The Node.js implementation: `simple-proxy.js`, `port-forward.js`, and `package.json`
   (git history preserves them).

### Changed

1. `LICENSE` — copyright notice extended from `2018` to `2018-2026`.
2. Logs are now timestamped in **UTC** (`YYYY-MM-DD HH:MM:SS.mmm`) rather than local
   time — better for servers and removes a class of timezone bugs.
3. Signal handling (SIGTERM/SIGINT) is registered **once** process-wide and fanned out
   via a shutdown channel, fixing the JS bug where each forwarded port installed a
   duplicate handler.
4. Dead-upstream auto-restart now triggers on a typed `TimedOut` error + connect
   timeout, replacing the JS's locale-fragile `startsWith("connect ETIMEDOUT")` check.
5. `README.md` rewritten for the Rust tool; the stale `npm install moment` step and the
   singular `source_port`/`target_port` keys (the code always used the plural
   `source_ports`/`target_ports`) are corrected.

---
## Version 1.2.0, 6/13/2026

> Initial documented release: a zero-dependency Node.js TCP socket proxy daemon plus a
> single-pair port-forward CLI, imported into this repo at v1.2.0.

### Added

1. `simple-proxy.js` — config-driven proxy daemon. Discovers the target VM IP by
   running a shell command from `proxy-config.json`, then forwards each configured
   `source_ports[i] → targetIp:target_ports[i]` pair. Gates every inbound connection
   through the `authorized` IP allow-list (exact match or `x.y.z.*` prefix);
   unauthorized remotes are destroyed before any data is forwarded.
2. `port-forward.js` — standalone CLI
   (`node port-forward.js source_ip:port target_ip:port`) that forwards a single
   static port pair for local VM work; no IP allow-list (a local-dev convenience).
3. 30-minute idle timeout (`IDLE_TIMEOUT`) on each client socket.
4. Auto-restart hook: on `ETIMEDOUT` to the configured `restart` target, the process
   exits non-zero so a process manager (systemd / docker-compose) restarts it.
5. Graceful shutdown on `SIGTERM` / `SIGINT` — ends tracked connections, then closes
   the server.
6. Timestamped logging via a local `consoleLog` helper, with a random per-process
   `INSTANCE_ID` and a per-connection 6-digit `sessionId`.
7. Sample configs: `proxy-config.json` (active), `proxy-config-multipass.json`
   (Multipass `ifconfig` IP discovery), and `proxy-config-hyperv.json`
   (Hyper-V `arp -a` IP discovery).
8. Zero runtime dependencies — Node.js standard library only
   (`net`, `child_process`, `crypto`, `fs`, `path`, `process`, `Intl`).
9. `LICENSE` (Apache-2.0) and `README.md`.

### Removed

N/A — initial documented release.

### Changed

N/A — initial documented release.
