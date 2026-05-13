# Deployment Guide -- syslog-mcp

Deployment patterns for syslog-mcp. Choose the method that fits your environment.

## Local development

```bash
cargo run -- serve mcp
```

Or via Justfile:

```bash
just dev
```

The server reads `config.toml` in the working directory. Syslog listens on `0.0.0.0:1514` and MCP on `0.0.0.0:3100`.

## Cargo install

```bash
cargo install syslog-mcp
syslog serve mcp
```

The binary reads `config.toml` from the current directory and accepts env var overrides.

The installed binary is `syslog`. Use `syslog mcp` for local MCP clients that require stdio. That mode is query-only: it reads `SYSLOG_MCP_DB_PATH`, exposes the MCP tools over stdin/stdout, and does not start syslog listeners, HTTP routes, retention purge, or storage-budget cleanup. Keep `syslog serve mcp` running somewhere for ingestion.

## Docker

The Docker image is daemon-focused: it runs `syslog serve mcp` for syslog ingest and HTTP MCP. Direct stdio is intended for host-installed binaries where the MCP client can launch `syslog mcp` and read the SQLite DB path directly.

### Build

Multi-stage Dockerfile: Rust 1.86 builder compiles the release binary, Debian bookworm-slim runtime copies only the binary.

```bash
just docker-build
# or: docker build -t syslog-mcp .
```

### Compose

```yaml
services:
  syslog-mcp:
    build: .
    container_name: syslog-mcp
    restart: unless-stopped
    user: "${SYSLOG_UID:-1000}:${SYSLOG_GID:-1000}"
    env_file:
      - path: ~/.claude-homelab/.env
        required: false
    ports:
      - "${SYSLOG_PORT:-1514}:1514/udp"
      - "${SYSLOG_PORT:-1514}:1514/tcp"
      - "${SYSLOG_MCP_PORT:-3100}:3100/tcp"
    volumes:
      - ${SYSLOG_MCP_DATA_VOLUME:-syslog-mcp-data}:/data
    healthcheck:
      test: ["CMD-SHELL", "curl -sf http://localhost:3100/health || exit 1"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 10s
    deploy:
      resources:
        limits:
          memory: 512M
          cpus: '1.0'
```

```bash
just up         # docker compose up -d
just down       # docker compose down
just restart    # docker compose restart
just logs       # docker compose logs -f
```

The installed `syslog` binary also provides guarded lifecycle diagnostics and mutations:

```bash
syslog compose doctor
syslog compose status --json
syslog compose pull
syslog compose up
syslog compose restart
syslog compose logs --tail 50
```

MCP exposes only redacted read-only Compose diagnostics (`compose_status`, `compose_doctor`). Lifecycle mutations remain CLI-only: ask the assistant to run `syslog compose ...` locally rather than invoking MCP actions.

### Container conventions

| Concern | Pattern |
| --- | --- |
| Base image | `rust:1.86-slim-bookworm` (builder) + `debian:bookworm-slim` (runtime) |
| User | Non-root, UID 1000 (`syslog`) |
| Health check | `curl -sf http://localhost:3100/health` every 30s |
| Data | Named volume mounted at `/data` |
| Network | External Docker network (`jakenet`) |
| Signals | Graceful shutdown on SIGTERM/SIGINT (tokio signal handler) |
| Config | No `config.toml` in image -- defaults + env vars only |

### Entrypoint

The entrypoint is minimal -- it delegates directly to the binary:

```bash
#!/bin/bash
set -euo pipefail
exec "$@"
```

All configuration is handled by the Rust binary's config loading (defaults + env vars).

## Port assignment

| Service | Default Port | Env Var | Protocol |
| --- | --- | --- | --- |
| Syslog receiver | 1514 | `SYSLOG_PORT` | UDP + TCP |
| MCP HTTP server | 3100 | `SYSLOG_MCP_PORT` | TCP |

Port 1514 is used instead of the standard syslog port 514 to avoid needing root or `CAP_NET_BIND_SERVICE`. Use iptables PREROUTING to redirect 514 to 1514 for devices that cannot be reconfigured.

## SWAG reverse proxy

See `docs/syslog.subdomain.conf` for a working nginx config that exposes MCP over HTTPS at `https://syslog-mcp.tootie.tv/mcp`.

The MCP endpoint uses RMCP Streamable HTTP in stateless JSON-response mode.
Clients use `POST /mcp`; `GET` and `DELETE` on `/mcp` are not supported after
auth succeeds.

## See also

- [ENV.md](ENV.md) -- environment variables
- [LOGS.md](LOGS.md) -- logging configuration
- [CONNECT.md](CONNECT.md) -- client connection methods
