# Agent Instructions — simple-proxy

## What This Project Is

A small, **zero-dependency TCP socket proxy** for Node.js. It was originally built
to let a user SSH into a Multipass Ubuntu VM from the host machine (or the Internet),
then generalized into a generic socket proxy that does NAT-style forwarding from a
host port to a guest-VM target IP and port. A companion CLI, `port-forward.js`,
forwards a single `source_ip:port → target_ip:port` pair for local VM work
(reaching Docker/Kubernetes services running inside a laptop VM from the host OS).

**Type:** CLI / utility tool (long-running daemon + one-shot CLI)
**Primary language:** JavaScript (Node.js, CommonJS) — verified on Node v22.12.0
**Framework / stack:** Node.js standard library only — `net`, `child_process`,
`crypto`, `fs`, `path`, `process`, `Intl.NumberFormat`. No external dependencies.

## Repository Structure

Flat single-package layout (no `src/`):

- `simple-proxy.js` — config-driven proxy daemon. Discovers the target VM IP by
  running a shell command from the config, then forwards each configured
  `source_ports[i] → targetIp:target_ports[i]` pair. Intended to run under PM2 or
  docker-compose.
- `port-forward.js` — standalone CLI: `node port-forward.js source_ip:port target_ip:port`.
  One static port pair; **no IP allow-list** (a local-dev convenience tool).
- `proxy-config.json` — the active config read by `simple-proxy.js`.
  `proxy-config-multipass.json` and `proxy-config-hyperv.json` are sample variants
  (Multipass `ifconfig` discovery vs. Hyper-V `arp -a` discovery).
- `package.json` — metadata; zero dependencies; no real build/test scripts.
- `README.md`, `LICENSE` (Apache-2.0).

## Conventions Observed

- CommonJS (`require`) with capitalized module aliases (`Net`, `Crypto`, `Fs`,
  `Path`, `Shell`).
- Logging goes through a local `consoleLog` helper formatted as
  `<local timestamp> [INSTANCE_ID] <message>`, where `INSTANCE_ID` is a random
  4-digit per-process id and each connection gets a random 6-digit `sessionId`.
- Legacy ES5 idioms throughout — `var`, loose `==`, and `for (key in array)`.
  These are intentional refactor targets (see `continuity.md` → Open Threads), not
  patterns to imitate in new code.
- The two `.js` files run directly under Node; there is no bundler or transpile step.

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

None. `npm test` is a placeholder that prints an error and exits 1. Verification is
manual: start the proxy and connect through it. A minimal smoke test is a candidate
once the code is refactored (see Open Threads).

## CI / CD

None. No `.github/workflows/`, CI config, or `Dockerfile`/`docker-compose.yml` are
committed, although the README recommends PM2 or docker-compose for deployment.

## Editing These Instructions

Only modify this file if the user explicitly asks to change the project
description, rules, or conventions. Treat it as stable configuration.
