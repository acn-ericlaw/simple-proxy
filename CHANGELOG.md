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
## Unreleased

> Repository tooling and housekeeping. No change to the proxy itself — the tool stays
> at v1.2.0 (source of truth: `package.json`).

### Added

1. `CHANGELOG.md` — this file, following the agent-memory changelog format.
2. AI-enablement via the **agent-memory** shared memory system (Mode A): `AGENTS.md`,
   per-vendor bootstrap pointers (`CLAUDE.md`, `GEMINI.md`, `.cursorrules`,
   `.windsurfrules`, `.github/copilot-instructions.md`), the repo `memory/` layer
   (`instructions.md`, `continuity.md`, `sessions/`, `archive/`), and the `.agent/`
   install manifest.

### Removed

N/A — additive.

### Changed

1. `LICENSE` — copyright notice extended from `2018` to `2018-2026`.

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
   exits non-zero so the process manager (PM2 / docker-compose) restarts it.
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
