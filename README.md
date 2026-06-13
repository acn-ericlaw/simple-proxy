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
