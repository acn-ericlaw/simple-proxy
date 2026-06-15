# simple-proxy

A minimalist **Layer-4 raw TCP forwarder** written in Rust + Tokio. It was originally
built to let a user SSH into a Multipass Ubuntu VM from the host machine (or the
Internet), then generalised into a small socket proxy that does NAT-style forwarding
from a host port to a guest-VM target `IP:port`.

Because it operates at Layer 4 (raw TCP), it is **protocol-agnostic** — SSH, HTTP,
HTTPS and anything else pass through as opaque bytes with no per-protocol configuration.
nginx is inspiration only; this is *not* an HTTP/L7 reverse proxy.

> **v2.0.0** is a ground-up rewrite from the original Node.js version to Rust + Tokio.
> The JSON config schema is unchanged, so existing `proxy-config.json` files keep
> working. See [`CHANGELOG.md`](./CHANGELOG.md).

## Build

Requires a recent Rust toolchain (`cargo`).

```sh
cargo build --release
# binary at: target/release/simple-proxy
```

## Usage

The binary has two subcommands:

```sh
# Config-driven daemon (default config: ./proxy-config.json)
simple-proxy serve [--config <path>]

# One-shot single port-pair forward
simple-proxy forward <src_ip:port> <dst_ip:port>
```

`forward` is handy for reaching a VM's services (Docker/Kubernetes, etc.) from the host
OS. To listen on all host interfaces, use `0.0.0.0` as the source IP. IPv6 literals must
be bracketed, e.g. `[::1]:22`.

```sh
simple-proxy forward 0.0.0.0:2222 192.168.64.7:22
```

## proxy-config.json

The `serve` daemon reads a JSON config. Either resolve the target IP dynamically via a
shell command (`discovery`), **or** point at a static `target_ip` — set exactly one.

Dynamic discovery (e.g. a Multipass VM):

```json
{
  "discovery": {
    "command": "multipass exec main ifconfig eth0",
    "tag": "inet",
    "index": 1
  },
  "source_ports": [22],
  "target_ports": [22],
  "authorized": ["192.168.1.*", "127.0.0.1"],
  "restart": 22
}
```

Static target (no discovery):

```json
{
  "target_ip": "192.168.64.7",
  "source_ports": [8080, 8443],
  "target_ports": [80, 443],
  "authorized": ["192.168.1.*"]
}
```

| Key | Meaning |
| --- | --- |
| `discovery` | Run `command` through the shell; from the first output line starting with `tag`, take whitespace field `index` as the target IP. Optional. |
| `target_ip` | Static upstream IP. Use this *instead of* `discovery`. |
| `source_ports` / `target_ports` | Parallel arrays: `source_ports[i]` on the host forwards to `target:target_ports[i]`. Must be equal length. |
| `authorized` | Inbound IP allow-list: exact IPs (`127.0.0.1`) and `x.y.z.*` wildcard prefixes. Connections from other IPs are dropped before any bytes are forwarded. An empty/absent list rejects everything. |
| `restart` | If a connect to this target port times out, the process exits non-zero so a process manager can restart it. Optional. |
| `idle_timeout_secs` | Per-connection idle timeout (default `1800` = 30 minutes). Optional. |

Two sample configs are included: `proxy-config-multipass.json` (Multipass `ifconfig`
discovery) and `proxy-config-hyperv.json` (Windows Hyper-V `arp -a` discovery).

> Note: the `serve` allow-list applies to the daemon only. `forward` has no allow-list —
> it is a local-dev convenience.

## Logging

Each line is `<UTC timestamp> [INSTANCE_ID] <message>`. `INSTANCE_ID` is a random
per-process id; each connection gets a random session id and reports rx/tx byte counts
on close. Timestamps are UTC (the original Node.js tool logged local time).

> Seeing `Remaining connections` stay above 0 after an HTTP request finished? That is
> usually client-side keep-alive, not a leak — see the
> [FAQ](#faq-a-connection-stays-open-after-my-http-request-finished) below.

## FAQ: "a connection stays open after my HTTP request finished"

This is **expected** and is **not** a proxy bug. It is HTTP **keep-alive** (persistent
connections), which clients use to reuse one TCP connection across requests.

`simple-proxy` is a Layer-4 forwarder: it keeps a TCP connection open for exactly as long
as **both** endpoints keep it open, and tears it down the instant **either** side closes
(client *or* upstream). It does not — and must not — close a connection that both ends are
deliberately holding open. So a lingering `Remaining connections = 1` after a request
completes reflects the **client's** keep-alive, not a leak in the proxy.

**Python `requests` / `urllib3`.** After `requests.get()` returns, the underlying socket is
*not* closed — it is kept alive for reuse, held open by the returned `Response` object
(`r.raw`). In CPython the socket closes only when that object is garbage-collected. So:

```python
r = requests.get("http://127.0.0.1:8080/info")  # connection opens, stays ESTABLISHED
r = requests.get("http://127.0.0.1:8080/info")  # rebinding r drops the old Response →
                                                 # old socket closes, a new one opens
del r                                            # last socket closes here
# (or it closes when the interpreter exits)
```

This is exactly the pattern in the proxy log: each new request closes the *previous*
session, and the final connection closes only when Python exits. To close eagerly, use a
`Session` as a context manager (`with requests.Session() as s: s.get(...)`) or call
`r.close()`. Verified locally with `lsof`: the socket survives the call's return and
disappears only on garbage collection.

**Browsers** behave the same way: they keep a pool of persistent connections open per host
(typically up to ~6) for reuse, closing them only after their own idle timeout. Seeing one
or more idle connections linger after a page loads is normal.

**When does a keep-alive connection actually close, then?** Whichever happens first:
the client closes it (GC / explicit close / browser pool timeout), the upstream server's
keep-alive timeout fires, or `simple-proxy`'s own `idle_timeout_secs` (default 30 min)
elapses with no traffic. Lower `idle_timeout_secs` if you want idle keep-alive connections
reclaimed sooner.

## Embedding: control-plane events (event-bus example)

When embedding the forwarder you can observe each connection's lifecycle — *opened*,
*closed* (with byte counts + reason), *rejected*, *upstream-unavailable* — by passing a
[`ConnObserver`](src/observer.rs) to `proxy::serve_listener_observed`. These are
**control-plane** signals emitted *around* the byte relay; the data path is untouched, and
the CLI binary uses a no-op observer by default (so it links no extra dependencies).

The [`event_bus_signaling`](examples/event_bus_signaling.rs) example bridges these events
onto the workspace's [`event-bus`](crates/event-bus) crate: a **broadcast** subscriber
prints a live monitor while a **work-queue** worker aggregates connection/byte metrics —
all driven by real traffic through the actual proxy.

```sh
cargo run --example event_bus_signaling
```

`event-bus` is a dev-dependency here, so it is **not** linked into the `simple-proxy`
binary — the demo keeps the shipping forwarder dependency-light.

## Running as a service

Run it under a process manager (e.g. **systemd**) or **docker-compose** so a crash or a
dead upstream restarts it. For example, a systemd unit:

```ini
[Service]
ExecStart=/usr/local/bin/simple-proxy serve --config /etc/simple-proxy/proxy-config.json
Restart=on-failure
```

Pair the `restart` config key with the manager's restart policy (systemd
`Restart=on-failure`, docker-compose `restart: on-failure`) so a dead upstream triggers a
restart.

## SSH security

If you expose a guest VM to the Internet, enforce certificate authentication — disable
SSH password login. Create a non-root user, install its public key in
`~/.ssh/authorized_keys`, and use an RSA key of at least 4,096 bits.
