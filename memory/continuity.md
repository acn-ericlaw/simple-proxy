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
- **status:** AI-enabled 2026-06-13 (fresh enable); stable at v1.2.0, refactor planned
- **last_enabled:** 2026-06-13
- **last_session:** (none yet)
- **last_review:** (none yet)
- **repo:** ~/sandbox/simple-proxy

## Stack & Tools

- **language:** JavaScript (Node.js, CommonJS) — verified on Node v22.12.0
- **runtime deps:** none — Node.js standard library only (`net`, `child_process`, `crypto`, `fs`, `path`, `process`, `Intl`)
- **version:** 1.2.0 (source of truth: `package.json`; `simple-proxy.js` APP_NAME agrees — no drift)
- **entry points:** `simple-proxy.js` (config-driven daemon), `port-forward.js` (CLI)
- **deploy:** PM2 (`pm2 start simple-proxy.js`) or docker-compose
- **remote:** GitHub `acn-ericlaw/simple-proxy`

## Architectural Invariants

> Hard constraints that must never change. These never decay (treated as `core`).

- Zero runtime dependencies — Node.js standard library only. Simplicity and
  performance are the project's whole value proposition; keep `package.json`
  dependencies empty.
  <!-- id: zero-runtime-deps | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: core -->
- `simple-proxy.js` gates every inbound connection through the `authorized` IP
  allow-list (exact match or `x.y.z.*` prefix); unauthorized remotes are destroyed
  before any data is forwarded.
  <!-- id: authorized-ip-allowlist | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: core -->

## Key Decisions

- Two entry points by design: `simple-proxy.js` (config file, dynamic VM-IP
  discovery via a shell command, multiple port pairs, IP allow-list, restart-on-dead-
  target) vs. `port-forward.js` (CLI args, one static port pair, no allow-list — a
  local-dev convenience).
  <!-- id: two-entry-points | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: active -->
- 30-minute idle timeout (`IDLE_TIMEOUT = 1800 * 1000`) on each client socket.
  <!-- id: idle-timeout-30m | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: active -->
- Auto-restart hook: in `simple-proxy.js`, if a target equals the configured
  `restart` port and the connection times out (`ETIMEDOUT`), the process `exit(1)`s
  so the process manager (PM2/docker) restarts it.
  <!-- id: restart-on-etimedout | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: active -->
- Graceful shutdown on SIGTERM/SIGINT: ends tracked connections, then closes the
  server.
  <!-- id: graceful-shutdown | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: active -->

## Conventions

- CommonJS, capitalized module aliases, timestamped `consoleLog` with random
  per-process `INSTANCE_ID` and per-connection 6-digit `sessionId`.
  <!-- id: logging-convention | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: active -->
- Legacy ES5 style (`var`, `==`, `for..in`) — slated for refactor, not to be
  imitated in new code.
  <!-- id: legacy-es5-style | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: active -->

## Open Threads

> Surfaced during the enable analysis on 2026-06-13. These are the refactor backlog.
> Source code was **not** modified during enablement.

- [ ] `package.json` `"main"` points to `socket-proxy.js`, which does not exist —
  the real entry points are `simple-proxy.js` / `port-forward.js`. Fix or remove.
  <!-- id: ot-package-main-wrong | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: active -->
- [ ] README "Library dependencies" tells users to `npm install` the `moment`
  package, but `package.json` has no dependencies and the code uses a custom
  timestamp helper (no moment). README is stale.
  <!-- id: ot-readme-moment-stale | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: active -->
- [ ] README's `proxy-config.json` example uses singular `source_port`/`target_port`
  and omits `restart`, but the code and shipped config files use plural
  `source_ports`/`target_ports` plus `restart`. Align the README with the real schema.
  <!-- id: ot-readme-config-keys-mismatch | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: active -->
- [ ] Code-quality refactor: `var` → `const`/`let`; `for (i in array)` leaks an
  implicit global `i` and iterates index strings — use `for...of`; loose `==` → `===`.
  <!-- id: ot-modernize-es | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: active -->
- [ ] Large duplication between `simple-proxy.js` and `port-forward.js`
  (`consoleLog`, `getLocalTimestamp`, `forwardPort`, graceful-shutdown) — extract a
  shared module.
  <!-- id: ot-extract-shared-module | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: active -->
- [ ] SIGTERM/SIGINT handlers are registered *inside* `forwardPort`, so when
  `simple-proxy.js` forwards multiple ports it attaches duplicate process-level signal
  listeners (one per port). Hoist to a single registration.
  <!-- id: ot-duplicate-signal-handlers | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: active -->
- [ ] No tests; `npm test` deliberately errors. Add a minimal smoke test once the
  code is refactored.
  <!-- id: ot-no-tests | created: 2026-06-13 | last_used: 2026-06-13 | uses: 1 | tier: active -->

## User Preferences

(none recorded yet — record ONLY what the user explicitly states; never infer)

## Team / Members

(none recorded yet)
