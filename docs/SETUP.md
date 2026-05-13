# Setup Guide -- syslog-mcp

Step-by-step instructions to get syslog-mcp running locally, in Docker, or as a Claude Code plugin.

## Prerequisites

| Dependency | Version | Purpose |
| --- | --- | --- |
| Rust | 1.86+ | Compiler toolchain |
| cargo | (bundled) | Build system and package manager |
| Docker | 24+ | Container deployment |
| Docker Compose | v2+ | Orchestration |
| just | latest | Task runner |
| openssl | any | Token generation |
| curl | any | Health checks |
| jq | any | JSON parsing (optional, for readable output) |

## 1. Clone the repository

```bash
git clone https://github.com/jmagar/syslog-mcp.git
cd syslog-mcp
```

## 2. Install Rust toolchain

If Rust is not installed:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable
```

## 3. Build

```bash
just build          # Debug build
just release        # Release build (optimized)
```

Or directly:

```bash
cargo build --release
```

## 4. Configure environment

```bash
cp .env.example .env
chmod 600 .env
```

Edit `.env` and set values as needed:

```bash
# Syslog listener
SYSLOG_HOST=0.0.0.0
SYSLOG_PORT=1514

# MCP server
SYSLOG_MCP_HOST=0.0.0.0
SYSLOG_MCP_PORT=3100

# Optional: enable bearer auth on /mcp endpoint
#   openssl rand -hex 32
SYSLOG_MCP_TOKEN=

# Storage
SYSLOG_MCP_DB_PATH=/data/syslog.db
SYSLOG_MCP_POOL_SIZE=4
SYSLOG_MCP_RETENTION_DAYS=90

# Log verbosity
RUST_LOG=info
```

See [CONFIG](CONFIG.md) for all environment variables.

## 5. Start locally

```bash
just dev
```

Or directly:

```bash
cargo run
```

The server reads `config.toml` in the working directory. Syslog listens on `0.0.0.0:1514` (UDP+TCP) and MCP on `0.0.0.0:3100` (HTTP).

## 6. Start via Docker

```bash
just up
```

Or manually:

```bash
docker compose up -d
```

Docker uses defaults and env vars exclusively -- `config.toml` is not copied into the image.

## 7. Verify

```bash
just health
```

Or:

```bash
curl http://localhost:3100/health
```

Expected response:

```json
{"status": "ok"}
```

Send a test syslog message and confirm it arrives:

```bash
logger -n localhost -P 1514 --tcp "test from $(hostname)"

curl -s -X POST http://localhost:3100/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"syslog","arguments":{"action":"tail","n":5}}}' | jq .
```

## 8. Install as Claude Code plugin

```bash
/plugin marketplace add jmagar/claude-homelab
/plugin install syslog-mcp @jmagar-claude-homelab
```

Configure the plugin with your MCP URL and optional API token when prompted.

## 9. Configure syslog sources

See [docs/](.) for per-host configuration:

- **Linux hosts**: rsyslog `/etc/rsyslog.d/99-remote.conf`
- **WSL hosts**: rsyslog with Tailscale IP
- **UniFi**: Settings > System > Advanced > Remote Syslog
- **ATT BGW-320**: Diagnostics > Syslog > Remote Syslog
- **Docker hosts**: optional docker-socket-proxy pull mode for container stdout/stderr logs

### Optional Docker host log ingest

If your hosts already run `docker-socket-proxy`, syslog-mcp can pull Docker container logs from those hosts without switching the Docker daemon to the syslog logging driver.

On each remote Docker host, expose only the read endpoints syslog-mcp needs:

```env
CONTAINERS=1
EVENTS=1
PING=1
VERSION=1
POST=0
```

Set `SYSLOG_DOCKER_HOSTS` to a comma-separated list of hostnames in `.env`:

```env
SYSLOG_DOCKER_INGEST_ENABLED=true
SYSLOG_DOCKER_HOSTS=squirts,tootie,dookie
```

Each hostname resolves to `http://<host>:2375`. Use only on trusted private networks (e.g. tailscale).

The ingest loop follows existing containers, listens for container start events, records checkpoints in SQLite, and reconnects with backoff if a host is unavailable. Remote containers still start normally if syslog-mcp is down because this path does not use Docker's daemon-level syslog logging driver.

Plain `http://` docker-socket-proxy URLs require `allow_insecure_http = true`. Use that only on trusted private networks, firewall the proxy so only syslog-mcp can connect, or put the proxy behind authenticated TLS. `CONTAINERS=1` exposes Docker's broader read-only container API to anything that can reach the proxy, not just the log endpoints syslog-mcp calls.

For Docker ingest integration testing, keep the default smoke test focused on UDP/TCP syslog and run Docker ingest as a separate fixture-backed check. Start syslog-mcp with `SYSLOG_DOCKER_INGEST_ENABLED=true` against a disposable docker-socket-proxy or mocked Docker HTTP endpoint, emit a unique marker from a short-lived container, then verify it with `search` or `tail`. Docker-ingested rows should report `source_ip` as `docker://<host>/<container>/<stream>`.

## Troubleshooting

### "Connection refused" on health check

- Confirm the server is running: `docker compose ps` or `ps aux | grep syslog-mcp`
- Verify `SYSLOG_MCP_PORT` matches the port you are curling
- If running in Docker, ensure port 3100 is published in `docker-compose.yml`

### "401 Unauthorized" on tool calls

- Verify `SYSLOG_MCP_TOKEN` in `.env` matches the token configured in your MCP client
- If behind a reverse proxy (SWAG), handle auth at the proxy layer and leave `SYSLOG_MCP_TOKEN` unset

### No syslog messages arriving

- Confirm the syslog port is reachable: `nc -zvu <host> 1514`
- Check iptables rules if redirecting 514 to 1514
- Verify rsyslog config on the sending host: `systemctl status rsyslog`
- Check Docker port mapping: `docker port syslog-mcp`

### Database errors at startup

- Ensure the data directory exists and is writable by UID 1000
- Check volume mounts: `docker inspect syslog-mcp | jq '.[0].Mounts'`
- Verify `SYSLOG_MCP_DB_PATH` points to a writable location

### Plugin not discovered by Claude Code

- Run `/plugin list` and confirm syslog-mcp appears
- Check `~/.claude/plugins/cache/` for the plugin directory
- Re-run `/plugin marketplace add jmagar/claude-homelab` to refresh

---

## OAuth Authentication

syslog-mcp supports Google OAuth 2.0 in addition to the static bearer token. See **[docs/OAUTH.md](OAUTH.md)** for the full setup guide, including:

- Google Console configuration (redirect URI, credentials)
- Required env vars (`SYSLOG_MCP_AUTH_MODE`, `SYSLOG_MCP_PUBLIC_URL`, Google client ID/secret)
- `config.toml` fields for allowlist, TTLs, and signing key path
- Operator FAQ (revoking users, rotating the JWT key)
