# Security Guardrails -- syslog-mcp

Safety and security patterns enforced across syslog-mcp.

## Credential management

### Storage

- All credentials in `.env` with `chmod 600` permissions
- Never commit `.env` or any file containing secrets
- Use `.env.example` as a tracked template with placeholder values only
- Generate tokens with `openssl rand -hex 32`

### Ignore files

`.gitignore` and `.dockerignore` must include:

```
.env
*.secret
credentials.*
*.pem
*.key
```

### Hook enforcement

Pre-commit hooks verify security invariants:

| Hook | Purpose |
| --- | --- |
| `sync-env.sh` | Ensures `.env.example` documents all variables read by the server |
| `fix-env-perms.sh` | Sets `.env` to `chmod 600` if present |


### Credential rotation

1. Generate new token: `openssl rand -hex 32`
2. Update `.env` with the new `SYSLOG_MCP_TOKEN` value
3. Restart the server: `just restart`
4. Update MCP client configuration with new token
5. Verify: `just health`

## Authentication

syslog-mcp has a single authentication boundary: MCP clients authenticating to the MCP HTTP server.

### Bearer token

When `SYSLOG_MCP_TOKEN` is set, all requests to `/mcp` require:

```
Authorization: Bearer {SYSLOG_MCP_TOKEN}
```

Token comparison uses `subtle::ConstantTimeEq` to prevent timing attacks.

### Unauthenticated by default

When `SYSLOG_MCP_TOKEN` is not set, the MCP endpoint is open. This is acceptable for LAN-only deployments but not recommended when exposed via reverse proxy.

### Health endpoint

`/health` is always unauthenticated -- required for Docker HEALTHCHECK, docker-compose probes, and SWAG liveness checks.

## Docker security

### Non-root execution

The container runs as non-root (UID/GID 1000):

```dockerfile
RUN groupadd --gid 1000 syslog && \
    useradd --uid 1000 --gid syslog --no-create-home --shell /sbin/nologin syslog
USER 1000:1000
```

Override with `SYSLOG_UID` and `SYSLOG_GID` in `docker-compose.yml`.

### No baked environment

The Docker image contains only two non-sensitive defaults:

```dockerfile
ENV RUST_LOG=info
ENV SYSLOG_MCP_DB_PATH=/data/syslog.db
```

No credentials are baked into the image. Verify with:

```bash
docker inspect syslog-mcp:latest | jq '.[0].Config.Env'
```

### Resource limits

The compose file sets memory and CPU limits:

```yaml
deploy:
  resources:
    limits:
      memory: 512M
      cpus: '1.0'
```

## Network security

### CORS restriction

CORS is restricted to `localhost:3100` and `127.0.0.1:3100`. MCP CLI clients (mcporter, curl) are not browser-based and ignore CORS entirely. This prevents malicious webpages from exfiltrating the log database via cross-origin browser fetch.

### Port 1514 vs 514

syslog-mcp listens on port 1514 (not 514) to avoid needing root or `CAP_NET_BIND_SERVICE`. Use iptables PREROUTING to redirect 514 to 1514 for devices that cannot be reconfigured:

```bash
sudo iptables -t nat -A PREROUTING -p udp --dport 514 -j REDIRECT --to-port 1514
sudo iptables -t nat -A PREROUTING -p tcp --dport 514 -j REDIRECT --to-port 1514
```

### SWAG reverse proxy

When exposing MCP over HTTPS via SWAG:
- Add auth at the proxy layer or set `SYSLOG_MCP_TOKEN`
- Add public reverse-proxy hostnames to `SYSLOG_MCP_ALLOWED_HOSTS` so RMCP Host validation accepts them
- See `docs/syslog.subdomain.conf` for a working nginx config

## Input handling

### Syslog message trust boundary

Syslog content (hostname, message, app_name) is untrusted user-controlled data. Any LAN host can UDP-spoof an arbitrary hostname. The `source_ip` field records the actual network sender address and is the only trustworthy network identity for a log entry.

### FTS5 query injection

`search_logs` passes user queries to SQLite FTS5. FTS5 query syntax is its own DSL (not SQL), so SQL injection is not possible via the query parameter. Invalid FTS5 syntax returns a database error, not a security vulnerability.

### SQL parameterization

All SQL queries use parameterized bindings (`params![]` and `named_params![]`). No user input is interpolated into SQL strings.

## Logging

- Never log credentials, tokens, or API keys
- Auth failure logs include method and path but never the submitted token value
- `RUST_LOG=debug` and `RUST_LOG=trace` are safe for development -- no secrets are emitted at any log level
