# Biz Tunnel

Bidirectional TCP port mapping for two sites where only the A-side edge host can initiate the cross-site connection.

## Topology

```text
B -> A:
B business client -> B relay exposed port -> tunnel -> A agent -> A target service

A -> B:
A business client -> A agent exposed port -> tunnel -> B relay -> B target service
```

The A-side `biz-agent` always dials the B-side `biz-relay`. Business applications are changed to use the local-side exposed address. No router, switch, TUN, TProxy, or source-IP preservation is required.

## Build

```bash
cargo build --release
```

The binaries are:

- `target/release/biz-relay`
- `target/release/biz-agent`
- `target/release/biz-tunnelctl`

## Run

B-side relay:

```bash
biz-relay --config /etc/biz-tunnel/relay.toml
```

A-side edge agent:

```bash
biz-agent --config /etc/biz-tunnel/agent.toml
```

Both config files must use the same `tunnel.id` and `tunnel.token`.
For file-based secrets, set `[security].token_file` and omit `[tunnel].token`.

## Configuration

See:

- `examples/relay.toml`
- `examples/agent.toml`

Direction rules:

- `b_to_a`: B-side clients connect to `expose_on_relay`; A-side agent dials `target_from_agent`.
- `a_to_b`: A-side clients connect to `expose_on_agent`; B-side relay dials `target_from_relay`.

Optional production controls:

- `[admin].token_file`: protects management APIs with `Authorization: Bearer <token>`.
- `[defaults].dial_timeout_secs`: default target dial timeout for new connections.
- `allowed_sources`: CIDR allowlist for clients connecting to the local exposed port.
- `[transport].mode = "tcp"`: TCP tunnel listener/dialer.
- `[transport].mode = "quic"`: QUIC tunnel over UDP. Each business TCP connection maps to one QUIC bidirectional stream.
- `[security]`: token, certificate, mTLS, and optional SHA-256 certificate fingerprint validation.

QUIC token mode requires:

- Relay: `security.cert` and `security.key`.
- Agent: `security.ca_cert` and `security.server_name`.
- Optional: `security.expected_peer_cert_sha256` to pin the peer certificate.

QUIC mTLS mode requires `security.ca_cert`, `security.cert`, `security.key`, and `security.server_name` on both sides.

## Management API

Each process exposes an HTTP API on `[admin].listen`:

- `GET /healthz`
- `GET /readyz`
- `GET /v1/services`
- `GET /v1/tunnel`
- `GET /v1/connections`
- `GET /v1/connections/{connection_id}`
- `POST /v1/services/reload`
- `GET /metrics`

If `[admin].token_file` is configured, every management endpoint except health/readiness requires:

```text
Authorization: Bearer <token>
```

`POST /v1/services/reload` reloads the original config file, diffs services by name, starts newly added listeners, stops removed listeners, and applies target/source-list changes to new connections. Changes to `role`, `tunnel.id`, tunnel listen/dial address, admin listen address, transport, or security require a process restart.

## Control Tool

```bash
biz-tunnelctl check-config --config /etc/biz-tunnel/relay.toml
biz-tunnelctl gen-token --out /etc/biz-tunnel/token
biz-tunnelctl cert-fingerprint --cert /etc/biz-tunnel/certs/server.pem
biz-tunnelctl reload --admin http://127.0.0.1:18080 --token-file /etc/biz-tunnel/admin-token
```

Install helper:

```bash
sudo packaging/install.sh
```

## Current Scope

Supported:

- TCP byte streams
- QUIC tunnel backend over UDP
- HTTP, gRPC, database protocols, and long-lived TCP connections
- Half-close semantics
- Shared-token tunnel authentication
- TLS server certificate validation for QUIC token mode
- mTLS for QUIC mode
- Optional peer certificate SHA-256 fingerprint pinning
- Agent reconnect
- Static service allowlist
- Source CIDR allowlist per exposed service
- Admin bearer-token protection
- Hot reload for service additions/removals/target changes
- Connection registry and per-service Prometheus metrics
- `biz-tunnelctl` check-config, gen-token, cert-fingerprint, and reload

Not supported in this build:

- UDP
- Transparent routing
- Source IP preservation
- Multi-agent high availability
- TLS/TCP data-plane backend
