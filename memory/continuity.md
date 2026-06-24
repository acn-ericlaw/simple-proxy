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

- **project:** simple-proxy  (Cargo workspace: proxy at root + `event-bus` crate under `crates/`)
- **status:** simple-proxy v2.0.0 is **realized & in maintenance** — committed & pushed to
  `origin/main`; CI gates (test/fmt/clippy) in place — toolchain now **pinned** to latest
  stable 1.96.0 via `rust-toolchain.toml` after a floating-stable lint break (`ci-toolchain-pinned`); 48 tests
  green; relay is symmetric-teardown relay-design-v3. **Active horizon: `event-bus` maturation** — the
  maintainer promoted the in-process `event-bus` crate to the new Vision (`vision-event-bus`);
  Phase-1 Blueprint open (see below). First gap **done**: the `event_bus_signaling` demo wires
  control-plane lifecycle events onto the bus via a no-op-by-default `ConnObserver` hook
  (`conn-observer-hook`); event-bus stays a dev-dependency so the binary is flume-free. event-bus is v0.1.0 today.
- **last_enabled:** 2026-06-13
- **last_session:** 2026-06-24 (GitHub Copilot) — drafted feedback note on agent-memory protocol skills experience. Prior (2026-06-24): recreated the greeting skill and synced adapters.
- **last_review:** 2026-06-15 (through 2026-06-15-172641.md)
- **last_invariant_check:** (none yet) — not due (10 session files < verify_invariants_every 20)
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
  <!-- id: layer-4-only | created: 2026-06-13 | last_used: 2026-06-15 | uses: 2 | tier: core -->
- Minimal dependency footprint — the proxy uses Tokio + serde + anyhow only (explicit
  tokio features, never `"full"`; no clap, no chrono). `flume` lives in the separate
  `event-bus` workspace crate, so the proxy binary links none of it. Minimalism is core.
  <!-- id: minimal-deps | created: 2026-06-13 | last_used: 2026-06-13 | uses: 3 | tier: core -->
- `serve` gates every inbound connection through the `authorized` IP allow-list (exact
  match or `x.y.z.*` prefix); unauthorized remotes are dropped before any data is
  forwarded. (`forward` mode has no allow-list by design.)
  <!-- id: authorized-ip-allowlist | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: core -->

## Key Decisions

- Created a new demo skill `greeting` that prints a friendly greeting message and local time, using the `$USER` environment variable. (Deleted)
  <!-- id: skill-greeting | created: 2026-06-24 | last_used: 2026-06-24 | uses: 1 | tier: superseded | origin: sessions/2026-06-24-162539.md -->
- Recreated the `greeting` skill with time-appropriate messaging (morning, afternoon, evening, night) using the `$USER` environment variable.
  <!-- id: skill-greeting-v2 | created: 2026-06-24 | last_used: 2026-06-24 | uses: 1 | tier: working | origin: sessions/2026-06-24-170937.md -->
- One binary, two subcommands: `serve` (config file, optional shell-command target-IP
  discovery OR static `target_ip`, multiple port pairs, IP allow-list, restart-on-dead-
  target) vs. `forward` (CLI args, one static pair, no allow-list — local-dev convenience).
  <!-- id: two-subcommands | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: archive-candidate -->
- Config `discovery` block is optional; omit it and set a static `target_ip`. Existing
  JSON schema (plural `source_ports`/`target_ports`, `authorized`, `restart`) preserved.
  <!-- id: optional-discovery | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: archive-candidate -->
- Symmetric teardown (v3): EITHER side closing (EOF) → `wu.shutdown()` + `wi.shutdown()` +
  break immediately. Eliminates the keep-alive linger bug (downstream closes, upstream has
  keep-alive → relay now exits at once instead of waiting 1800s). `c2u_open` flag removed.
  `larger_payload_round_trips` test updated to use `read_exact` + defer `OwnedWriteHalf`
  drop until after the read (so FIN isn't sent until data is received). New regression test:
  `client_close_propagates_to_upstream`. All 8 integration tests pass. Clippy/fmt clean.
  <!-- id: relay-design-v3 | created: 2026-06-15 | last_used: 2026-06-15 | uses: 4 | tier: active | origin: sessions/2026-06-15-162002.md | supersedes: relay-design-v2 -->
- A client keep-alive connection lingering after an HTTP request completes is EXPECTED, not a
  proxy leak. The proxy (relay-design-v3) tears down promptly when EITHER side closes; with
  keep-alive, neither side closes. Python `requests` holds the socket open via the returned
  `Response` (`r.raw`); it closes on CPython GC (rebind/`del`/exit), the server's keep-alive
  timeout, or the proxy's `idle_timeout_secs`. Browsers do the same via their connection pool.
  Reproduced locally with `lsof`. Documented as a README FAQ ("a connection stays open after
  my HTTP request finished").
  <!-- id: keepalive-client-linger-expected | created: 2026-06-15 | last_used: 2026-06-15 | uses: 1 | tier: working | origin: sessions/2026-06-15-165645.md -->
- Idle timeout default 1800s (30 min), configurable via `idle_timeout_secs`.
  <!-- id: idle-timeout-30m | created: 2026-06-13 | last_used: 2026-06-15 | uses: 2 | tier: active -->
- Auto-restart hook: a connect timeout to the configured `restart` target port triggers
  `process::exit(1)` so a process manager (systemd/docker) restarts. Detected via typed `io::ErrorKind::TimedOut`
  + a bounded connect timeout (not a locale-fragile error-string match).
  <!-- id: restart-on-timeout | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: archive-candidate -->
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
  <!-- id: event-bus-module | created: 2026-06-13 | last_used: 2026-06-13 | uses: 2 | tier: active -->
- Control-plane lifecycle hook (realizes `bp-eb-proxy-signaling-demo`): the `observer` module
  (`ConnEvent` {Rejected, UpstreamUnavailable, Opened, Closed{rx,tx,reason}}, `ConnObserver`
  trait, `NoopObserver`) + `proxy::serve_listener_observed`. Events fire AROUND the relay
  (control plane only — `relay()` byte path untouched, `layer-4-only` preserved). The default
  path (`serve_listener`, the CLI binary) uses `NoopObserver` and links nothing extra;
  `event-bus` is a **dev-dependency** used ONLY by the `event_bus_signaling` example, so the
  shipping binary stays flume-free (`minimal-deps`). Example shows broadcast (live monitor) +
  work-queue (metrics) delivery driven by real proxy traffic. Now part of the stable embedding
  surface (`bp-embedding-surface`); `ExitReason::label()` added for stable close-reason strings.
  <!-- id: conn-observer-hook | created: 2026-06-15 | last_used: 2026-06-15 | uses: 1 | tier: working | origin: sessions/2026-06-15-190408.md -->

## Conventions

- `logln!` macro → `<UTC timestamp> [INSTANCE_ID] <message>`; random per-process
  `INSTANCE_ID` (4-digit) + per-connection `sessionId` (6-digit). Logs are UTC by design
  (the JS version logged local time). Build the full line, then one `println!`.
  <!-- id: logging-convention | created: 2026-06-13 | last_used: 2026-06-13 | uses: 2 | tier: active -->
- Idiomatic async Rust; `anyhow::Result` at the edges, per-connection errors contained;
  typed `io::ErrorKind` matching. Gate on `cargo fmt --check` + `cargo clippy`.
  <!-- id: rust-style | created: 2026-06-13 | last_used: 2026-06-15 | uses: 2 | tier: active -->
- Toolchain is **pinned** via `rust-toolchain.toml` (`channel = "1.96.0"` — the current latest
  stable; components `rustfmt`/`clippy`) — the single source of truth for local builds AND CI.
  CI honors it with `rustup show` (NOT `dtolnay/rust-toolchain@stable`, which ignores the toml
  and floats). This exists because floating stable + `RUSTFLAGS: -D warnings` once turned a
  newly-promoted clippy lint into a no-code-change CI break. Bump `channel` deliberately — and
  re-run fmt/clippy/test under the new version — to adopt new lints (done for 1.96.0 on
  2026-06-15: all gates green).
  <!-- id: ci-toolchain-pinned | created: 2026-06-15 | last_used: 2026-06-15 | uses: 1 | tier: working | origin: sessions/2026-06-15-183705.md -->

## Open Threads

- [ ] (skills-layer test) Verify the `hello-world` portable skill on a **different machine via
  Gemini CLI** (maintainer, pending). The neutral `agent-skills/hello-world/SKILL.md` travels
  via git; the per-vendor adapters (`.claude/skills/`, `.gemini/commands/`, `.cursor/rules/`)
  are **gitignored** and do NOT travel. So on the other machine: ask Gemini "run hello world"
  → the `GEMINI.md`→`AGENTS.md` **baseline** reads the neutral skill (proves vendor-neutral
  portability with zero committed Gemini files). For the `/hello-world` slash command,
  regenerate the Gemini adapter there first (adapters are per-machine).
  <!-- id: ot-skill-crossmachine-test | created: 2026-06-16 | last_used: 2026-06-16 | uses: 1 | tier: working | origin: 2026-06-16-170420 -->

- [x] (skills-layer test) Test-drove the **v4.3.0 session-close skills safety check**. A skill
  authored natively in `.claude/skills/session-close-demo/` (no matching `agent-skills/`) was
  correctly detected as **stranded** at session close and **adopted**: promoted to the neutral
  `agent-skills/session-close-demo/SKILL.md` + "sync skill adapters" regenerated all three
  adapters as pointers (the hand-authored Claude file replaced). Verified: neutral skill TRACKED
  (only path `git status` shows), all three adapters IGNORED (`.gitignore` `.claude/`/`.gemini/`/
  `.cursor/`) — they don't travel. `session-close-demo` was a **throwaway fixture**, **removed
  after the test** (the safety check + adopt are verified; this session log is the record).
  <!-- id: ot-session-close-safety-check-verified | created: 2026-06-16 | last_used: 2026-06-16 | uses: 1 | tier: working | origin: sessions/2026-06-16-182943.md -->

- [x] (blueprint / human gate) The `vision-simple-proxy` gap closed (all five gaps `[x]`).
  **Resolved 2026-06-15:** the maintainer confirmed that Vision **realized** and chose to
  promote the in-process `event-bus` crate to the new Vision (`vision-event-bus`, in
  `memory/vision.md`) — scope: mature the in-process library (bounded/backpressure,
  typed-optional, observable, route lifecycle, versioned); simple-proxy stays independent /
  maintenance and is NOT the bus's consumer. Realized `vision-simple-proxy` archived flagged
  superseded. New Phase-1 Blueprint opened below. Raised by the 2026-06-15 review.
  <!-- id: ot-vision-gap-closed-rehorizon | created: 2026-06-15 | last_used: 2026-06-15 | uses: 1 | tier: working | origin: sessions/2026-06-15-172641.md -->

- [x] (ci-bug) GitHub Actions CI was red on every recent push (even memory-only commits):
  clippy `items_after_test_module` error at `src/proxy.rs` under `-D warnings`, surfaced when
  floating `dtolnay/rust-toolchain@stable` rolled 1.95 → 1.96. **Fixed 2026-06-15:** reordered
  `ConnGuard` above its test module; pinned the toolchain via `rust-toolchain.toml` + `rustup`
  in CI (see `ci-toolchain-pinned`). Confirmed **green on GitHub** — run 27568410747 on tip
  `bd65b0e` passed fmt/clippy/test under the pinned 1.96.0.
  <!-- id: ot-ci-items-after-test-module | created: 2026-06-15 | last_used: 2026-06-15 | uses: 1 | tier: working | origin: sessions/2026-06-15-183705.md -->

- [x] (clarification) User reported a connection lingering after `requests.get()` returned
  (also seen in browsers). Investigated → expected client keep-alive + CPython GC, not a proxy
  bug. Documented as a README FAQ. See `keepalive-client-linger-expected`.
  <!-- id: ot-keepalive-linger-faq | created: 2026-06-15 | last_used: 2026-06-15 | uses: 1 | tier: working | origin: sessions/2026-06-15-165645.md -->
  <!-- review-note: not named in any session ## Memory References (the origin session 2026-06-15-165645 listed only keepalive-client-linger-expected under Created). Logs are immutable; kept seeded uses:1/last_used per origin rather than fabricate a ledger event. -->

- [x] (bug) Upstream close did not tear down idle client connections promptly — fixed in
  commit `03e58b3` with asymmetric relay teardown. Regression tests added.
  <!-- id: ot-upstream-close-bug | created: 2026-06-15 | last_used: 2026-06-15 | uses: 1 | tier: active | origin: sessions/2026-06-15-050643.md -->

- [x] (bug) Client close did not tear down keep-alive upstream connections promptly (mirror
  of upstream-close bug) — fixed with symmetric relay teardown (relay-design-v3). Regression
  test `client_close_propagates_to_upstream` added.
  <!-- id: ot-client-close-keepalive-bug | created: 2026-06-15 | last_used: 2026-06-15 | uses: 2 | tier: active | origin: sessions/2026-06-15-162002.md -->

- [x] (vision-bootstrap) Confirmed the Vision in memory/vision.md — target / success criteria / non-goals set; Blueprint derived below.
  <!-- id: ot-vision-bootstrap | created: 2026-06-14 | last_used: 2026-06-15 | uses: 1 | tier: active | origin: sessions/2026-06-15-044225.md -->

### Blueprint — Phase 1: mature the in-process event-bus  (Vision↔Current-State gap — each serves: vision-event-bus)

> Opened 2026-06-15 by the re-horizon human gate (`ot-vision-gap-closed-rehorizon`). Scope
> confirmed by the maintainer: mature `crates/event-bus` (v0.1.0) as a production-grade,
> minimal, embeddable **in-process** library. Networking is explicitly out of scope (a future
> Vision, not this one). simple-proxy is NOT the consumer — the bus proves itself elsewhere.

- [ ] (blueprint) **Bounded delivery + backpressure.** Replace the unconditional unbounded
  flume channels with a configurable bounded option and an explicit overflow policy
  (block / drop-oldest / reject); `Delivered` reports rejection. serves: vision-event-bus
  <!-- id: bp-eb-bounded-backpressure | created: 2026-06-15 | last_used: 2026-06-15 | uses: 1 | tier: working | origin: sessions/2026-06-15-181756.md -->
- [ ] (blueprint) **Typed-optional payload layer.** An opt-in (feature-gated) typed/serde
  API over the `Vec<u8>` core — `publish_typed::<T>` / typed receivers — without forcing serde
  on byte-level users. Preserves minimal-deps. serves: vision-event-bus
  <!-- id: bp-eb-typed-payloads | created: 2026-06-15 | last_used: 2026-06-15 | uses: 1 | tier: working | origin: sessions/2026-06-15-181756.md -->
- [ ] (blueprint) **Observability.** Per-route metrics exposed through a stable API: queue
  depth, delivered / dropped counts, subscriber & worker counts. serves: vision-event-bus
  <!-- id: bp-eb-observability | created: 2026-06-15 | last_used: 2026-06-15 | uses: 1 | tier: working | origin: sessions/2026-06-15-181756.md -->
- [ ] (blueprint) **Route lifecycle.** Teardown/reclaim of routes & queues so the `broadcast`
  and `queues` maps don't grow unbounded (today only dead broadcast subscribers are pruned;
  queue channels and empty route entries persist forever). serves: vision-event-bus
  <!-- id: bp-eb-route-lifecycle | created: 2026-06-15 | last_used: 2026-06-15 | uses: 1 | tier: working | origin: sessions/2026-06-15-181756.md -->
- [x] (blueprint) **simple-proxy control-plane signaling demo (experiment).** Wire the
  event-bus into a simple-proxy *demo* to signal connection-lifecycle events (socket open /
  close, etc.) on the **control plane** — NOT the byte data path. Must be a separate example /
  feature-gated artifact so the shipping `simple-proxy` binary stays flume-free
  (`minimal-deps` core invariant) and the relay stays direct (`layer-4-only`). A learning
  vehicle to refine the bus before a real-world consumer. **Constraint maintainer-confirmed
  2026-06-15** (separate artifact, not flume in the shipping binary — do not re-open without
  an explicit `minimal-deps` invariant change). **Done 2026-06-15:** added a no-op-by-default
  `ConnObserver` hook (`conn-observer-hook`) + the `event_bus_signaling` example (event-bus
  as a dev-dependency only). 50 tests green; demo runs. serves: vision-event-bus
  <!-- id: bp-eb-proxy-signaling-demo | created: 2026-06-15 | last_used: 2026-06-15 | uses: 1 | tier: working | origin: sessions/2026-06-15-181756.md -->
- [ ] (blueprint) **Real-world consumer (after the experiment).** ≥1 real integration that is
  NOT simple-proxy and NOT an example/demo — a genuine consumer that exercises the *refined*
  bus in anger. Informed by, and sequenced after, the signaling-demo experiment above.
  serves: vision-event-bus
  <!-- id: bp-eb-real-consumer | created: 2026-06-15 | last_used: 2026-06-15 | uses: 1 | tier: working | origin: sessions/2026-06-15-181756.md -->
- [ ] (blueprint) **Stable, versioned, published.** Settle the 1.0 public surface
  (`#[non_exhaustive]` where apt), docs + CHANGELOG, and publish (crates.io or internal
  registry) so a larger system can depend on it without forks. serves: vision-event-bus
  <!-- id: bp-eb-publish-stable | created: 2026-06-15 | last_used: 2026-06-15 | uses: 1 | tier: working | origin: sessions/2026-06-15-181756.md -->

### Blueprint (realized) — vision-simple-proxy  (all closed 2026-06-15; left in place until the review sweeps them past archive_window 20)

- [x] (blueprint) Commit & baseline the v2.0.0 Rust workspace — committed in `6cd80cd`; all tests green. serves: vision-simple-proxy
  <!-- id: bp-commit-baseline | created: 2026-06-14 | last_used: 2026-06-15 | uses: 3 | tier: active | origin: sessions/2026-06-15-044225.md -->
- [x] (blueprint) Prove production-grade reliability — graceful shutdown tested (`graceful_shutdown_stops_accept_loop`); auto-restart decision logic unit-tested (`proxy::tests`); `ConnGuard` RAII counter tested; relay teardown paths fully covered. Actual `process::exit(1)` + process-manager cycle requires manual/deploy verification. serves: vision-simple-proxy
  <!-- id: bp-prove-reliability | created: 2026-06-14 | last_used: 2026-06-15 | uses: 3 | tier: active | origin: sessions/2026-06-15-044225.md -->
- [x] (blueprint) Mature crates/event-bus into a standalone, documented, versioned library with ≥1 real consumer beyond the demo — README expanded, `pipeline` example added (3-stage work-queue pipeline; real architectural consumer). serves: vision-simple-proxy
  <!-- id: bp-event-bus-standalone | created: 2026-06-14 | last_used: 2026-06-15 | uses: 3 | tier: active | origin: sessions/2026-06-15-044225.md -->
- [x] (blueprint) Define a stable embedding surface (public API + config) so the proxy/event-bus drop into a larger system without forks — `lib.rs` documents stable API vs. binary helpers; `#[non_exhaustive]` added to `ExitReason`, `RelayStats`, `Config`, `Discovery`. serves: vision-simple-proxy
  <!-- id: bp-embedding-surface | created: 2026-06-14 | last_used: 2026-06-15 | uses: 3 | tier: active | origin: sessions/2026-06-15-044225.md -->
- [x] (blueprint) Add CI gates (cargo test/fmt/clippy) — `.github/workflows/ci.yml` committed; runs on push/PR to main. serves: vision-simple-proxy
  <!-- id: bp-ci-gates | created: 2026-06-14 | last_used: 2026-06-15 | uses: 4 | tier: active | origin: sessions/2026-06-15-044225.md -->

> The v2.0.0 Rust rewrite (2026-06-13) resolved the entire JS-era refactor backlog below.

- [x] `package.json` `"main"` pointed to a non-existent `socket-proxy.js`. Resolved:
  `package.json` removed (now a Cargo project).
  <!-- id: ot-package-main-wrong | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: archive-candidate -->
- [x] README "Library dependencies" told users to `npm install moment` (stale). Resolved:
  README rewritten for the Rust tool.
  <!-- id: ot-readme-moment-stale | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: archive-candidate -->
- [x] README config example used singular `source_port`/`target_port`. Resolved: README
  now documents the real plural `source_ports`/`target_ports` schema.
  <!-- id: ot-readme-config-keys-mismatch | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: archive-candidate -->
- [x] Legacy ES5 idioms (`var`, `==`, `for..in`). Resolved: rewritten in Rust.
  <!-- id: ot-modernize-es | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: archive-candidate -->
- [x] Code duplication between the two `.js` files. Resolved: shared `relay`/`log`/etc.
  modules in one crate.
  <!-- id: ot-extract-shared-module | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: archive-candidate -->
- [x] Duplicate SIGTERM/SIGINT handlers (one per forwarded port). Resolved: signals
  registered once, fanned out via the shutdown channel.
  <!-- id: ot-duplicate-signal-handlers | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: archive-candidate -->
- [x] No tests. Resolved: `cargo test` unit + integration suite added.
  <!-- id: ot-no-tests | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: archive-candidate -->

## User Preferences

(none recorded yet — record ONLY what the user explicitly states; never infer)

## Team / Members

(none recorded yet)
