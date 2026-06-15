# Continuity — simple-proxy

> Shared ground truth for project state across all agents and sessions.
> Update at the end of every session. Never delete — only archive (see `REVIEW.md`).
>
> Each fact carries a metadata footer in an HTML comment, maintained by the review
> ritual — invisible when rendered, read/written by agents:
> `<!-- id: kebab-id | created: YYYY-MM-DD | last_used: YYYY-MM-DD | uses: N | tier: active -->`
> See `.agent/schema.md` for the fields and `memory/decay-policy.md` for the windows.

---

## Project State

- **project:** simple-proxy
- **status:** Rust + Tokio v2.0.0; now a Cargo workspace (proxy at root + `event-bus` crate under `crates/`); builds/tests green, not yet committed (2026-06-13)
- **last_enabled:** 2026-06-13
- **last_session:** 2026-06-15 (Claude Code)
- **last_review:** (none yet)
- **last_invariant_check:** (none yet)
- **repo:** ~/sandbox/simple-proxy

## Stack & Tools

> Canonical live home for the current stack — language version, dependencies, tool
> versions. `instructions.md` keeps only a high-level descriptor and points here.

- **language:** Rust (edition 2021) — built with cargo 1.95
- **runtime deps:** proxy — tokio (rt-multi-thread/net/io-util/time/process/signal/sync/macros), serde + serde_json, anyhow. `event-bus` crate — flume (+ tokio dev-only for its example/test)
- **version:** 2.0.0 (source of truth: `Cargo.toml`)
- **entry points:** single binary `simple-proxy` with `serve` (daemon) and `forward` (one-shot) subcommands
- **deploy:** a process manager (e.g. systemd, `Restart=on-failure`) or docker-compose, paired with the `restart` config key
- **remote:** GitHub `acn-ericlaw/simple-proxy`

## Architectural Invariants

> Hard constraints that must never change. These never decay (treated as `core`).

- Layer-4 raw TCP forwarder only — protocol-agnostic (SSH/HTTP/HTTPS pass through as
  bytes). nginx is inspiration only; do NOT turn this into an HTTP/L7 proxy.
  <!-- id: layer-4-only | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: core -->
- Minimal dependency footprint — the proxy uses Tokio + serde + anyhow only (explicit
  tokio features, never `"full"`; no clap, no chrono). `flume` lives in the separate
  `event-bus` workspace crate, so the proxy binary links none of it. Minimalism is core.
  <!-- id: minimal-deps | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: core -->
- `serve` gates every inbound connection through the `authorized` IP allow-list (exact
  match or `x.y.z.*` prefix); unauthorized remotes are dropped before any data is
  forwarded. (`forward` mode has no allow-list by design.)
  <!-- id: authorized-ip-allowlist | created: 2026-06-13 | last_used: 2026-06-13 | uses: 2 | tier: core -->

## Key Decisions

- One binary, two subcommands: `serve` (config file, optional shell-command target-IP
  discovery OR static `target_ip`, multiple port pairs, IP allow-list, restart-on-dead-
  target) vs. `forward` (CLI args, one static pair, no allow-list — local-dev convenience).
  <!-- id: two-subcommands | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: working -->
- Config `discovery` block is optional; omit it and set a static `target_ip`. Existing
  JSON schema (plural `source_ports`/`target_ports`, `authorized`, `restart`) preserved.
  <!-- id: optional-discovery | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: working -->
- Core relay: a single `tokio::select!` loop over borrowing `TcpStream::split()`, each
  read wrapped in `tokio::time::timeout(idle, ..)` for reset-on-activity idle, with TCP
  half-close on EOF and rx/tx byte counters. (Not `copy_bidirectional` — it can't idle-reset.)
  <!-- id: relay-design | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: superseded | superseded-by: relay-design-v2 -->
- Asymmetric teardown (v2): upstream close → `wi.shutdown()` + break immediately (client
  sees EOF at once; idle connections don't linger). Client write-close → `wu.shutdown()` +
  continue draining upstream→client (preserves HTTP response / large-payload delivery).
  `u2c_open` flag removed; only `c2u_open` remains. Commit `03e58b3`.
  <!-- id: relay-design-v2 | created: 2026-06-15 | last_used: 2026-06-15 | uses: 1 | tier: superseded | superseded-by: relay-design-v3 | origin: sessions/2026-06-15-050643.md | supersedes: relay-design -->
- Symmetric teardown (v3): EITHER side closing (EOF) → `wu.shutdown()` + `wi.shutdown()` +
  break immediately. Eliminates the keep-alive linger bug (downstream closes, upstream has
  keep-alive → relay now exits at once instead of waiting 1800s). `c2u_open` flag removed.
  `larger_payload_round_trips` test updated to use `read_exact` + defer `OwnedWriteHalf`
  drop until after the read (so FIN isn't sent until data is received). New regression test:
  `client_close_propagates_to_upstream`. All 8 integration tests pass. Clippy/fmt clean.
  <!-- id: relay-design-v3 | created: 2026-06-15 | last_used: 2026-06-15 | uses: 1 | tier: working | origin: sessions/2026-06-15-162002.md | supersedes: relay-design-v2 -->
- Idle timeout default 1800s (30 min), configurable via `idle_timeout_secs`.
  <!-- id: idle-timeout-30m | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: active -->
- Auto-restart hook: a connect timeout to the configured `restart` target port triggers
  `process::exit(1)` so a process manager (systemd/docker) restarts. Detected via typed `io::ErrorKind::TimedOut`
  + a bounded connect timeout (not a locale-fragile error-string match).
  <!-- id: restart-on-timeout | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: active -->
- Graceful shutdown via a single `tokio::sync::watch` channel; SIGTERM/SIGINT registered
  ONCE process-wide and fanned out (fixes the JS per-port duplicate-handler bug).
  <!-- id: graceful-shutdown | created: 2026-06-13 | last_used: 2026-06-13 | uses: 2 | tier: active -->
- `event-bus` crate (`crates/event-bus/`) — a standalone, reusable named-route event bus
  on flume: `Vec<u8>` payloads; broadcast pub/sub (`subscribe`) AND work-queue (`worker`)
  delivery that can coexist per route; `publish` is sync/non-blocking. Its own workspace
  crate so the `simple-proxy` binary links no flume; deliberately NOT in the proxy data
  path (the byte relay stays direct) — preparation for a larger event-bus project. Demo:
  `cargo run -p event-bus --example event_bus_demo`. Modeled on
  `~/sandbox/rust/rust_event_bus_example` but generalized (bytes, both delivery modes).
  <!-- id: event-bus-module | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: working -->

## Conventions

- `logln!` macro → `<UTC timestamp> [INSTANCE_ID] <message>`; random per-process
  `INSTANCE_ID` (4-digit) + per-connection `sessionId` (6-digit). Logs are UTC by design
  (the JS version logged local time). Build the full line, then one `println!`.
  <!-- id: logging-convention | created: 2026-06-13 | last_used: 2026-06-13 | uses: 2 | tier: active -->
- Idiomatic async Rust; `anyhow::Result` at the edges, per-connection errors contained;
  typed `io::ErrorKind` matching. Gate on `cargo fmt --check` + `cargo clippy`.
  <!-- id: rust-style | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: working -->

## Open Threads

- [x] (bug) Upstream close did not tear down idle client connections promptly — fixed in
  commit `03e58b3` with asymmetric relay teardown. Regression tests added.
  <!-- id: ot-upstream-close-bug | created: 2026-06-15 | last_used: 2026-06-15 | uses: 1 | tier: working | origin: sessions/2026-06-15-050643.md -->

- [x] (bug) Client close did not tear down keep-alive upstream connections promptly (mirror
  of upstream-close bug) — fixed with symmetric relay teardown (relay-design-v3). Regression
  test `client_close_propagates_to_upstream` added.
  <!-- id: ot-client-close-keepalive-bug | created: 2026-06-15 | last_used: 2026-06-15 | uses: 1 | tier: working | origin: sessions/2026-06-15-162002.md -->

- [x] (vision-bootstrap) Confirmed the Vision in memory/vision.md — target / success criteria / non-goals set; Blueprint derived below.
  <!-- id: ot-vision-bootstrap | created: 2026-06-14 | last_used: 2026-06-14 | uses: 1 | tier: working -->

### Blueprint  (Vision↔Current-State gap — each serves: vision-simple-proxy)

- [x] (blueprint) Commit & baseline the v2.0.0 Rust workspace — committed in `6cd80cd`; all tests green. serves: vision-simple-proxy
  <!-- id: bp-commit-baseline | created: 2026-06-14 | last_used: 2026-06-14 | uses: 1 | tier: working | origin: sessions/2026-06-15-044225.md -->
- [x] (blueprint) Prove production-grade reliability — graceful shutdown tested (`graceful_shutdown_stops_accept_loop`); auto-restart decision logic unit-tested (`proxy::tests`); `ConnGuard` RAII counter tested; relay teardown paths fully covered. Actual `process::exit(1)` + process-manager cycle requires manual/deploy verification. serves: vision-simple-proxy
  <!-- id: bp-prove-reliability | created: 2026-06-14 | last_used: 2026-06-14 | uses: 1 | tier: working | origin: sessions/2026-06-15-044225.md -->
- [ ] (blueprint) Mature crates/event-bus into a standalone, documented, versioned library with ≥1 real consumer beyond the demo. serves: vision-simple-proxy
  <!-- id: bp-event-bus-standalone | created: 2026-06-14 | last_used: 2026-06-14 | uses: 1 | tier: working | origin: sessions/2026-06-15-044225.md -->
- [ ] (blueprint) Define a stable embedding surface (public API + config) so the proxy/event-bus drop into a larger system without forks. serves: vision-simple-proxy
  <!-- id: bp-embedding-surface | created: 2026-06-14 | last_used: 2026-06-14 | uses: 1 | tier: working | origin: sessions/2026-06-15-044225.md -->
- [x] (blueprint) Add CI gates (cargo test/fmt/clippy) — `.github/workflows/ci.yml` committed; runs on push/PR to main. serves: vision-simple-proxy
  <!-- id: bp-ci-gates | created: 2026-06-14 | last_used: 2026-06-14 | uses: 1 | tier: working | origin: sessions/2026-06-15-044225.md -->

> The v2.0.0 Rust rewrite (2026-06-13) resolved the entire JS-era refactor backlog below.

- [x] `package.json` `"main"` pointed to a non-existent `socket-proxy.js`. Resolved:
  `package.json` removed (now a Cargo project).
  <!-- id: ot-package-main-wrong | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: active -->
- [x] README "Library dependencies" told users to `npm install moment` (stale). Resolved:
  README rewritten for the Rust tool.
  <!-- id: ot-readme-moment-stale | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: active -->
- [x] README config example used singular `source_port`/`target_port`. Resolved: README
  now documents the real plural `source_ports`/`target_ports` schema.
  <!-- id: ot-readme-config-keys-mismatch | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: active -->
- [x] Legacy ES5 idioms (`var`, `==`, `for..in`). Resolved: rewritten in Rust.
  <!-- id: ot-modernize-es | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: active -->
- [x] Code duplication between the two `.js` files. Resolved: shared `relay`/`log`/etc.
  modules in one crate.
  <!-- id: ot-extract-shared-module | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: active -->
- [x] Duplicate SIGTERM/SIGINT handlers (one per forwarded port). Resolved: signals
  registered once, fanned out via the shutdown channel.
  <!-- id: ot-duplicate-signal-handlers | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: active -->
- [x] No tests. Resolved: `cargo test` unit + integration suite added.
  <!-- id: ot-no-tests | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: active -->

## User Preferences

(none recorded yet — record ONLY what the user explicitly states; never infer)

## Team / Members

(none recorded yet)
