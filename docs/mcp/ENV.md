# Environment Variable Reference -- syslog-mcp

Concise reference. See [CONFIG.md](../CONFIG.md) for full documentation including config.toml overlay and validation rules.

## Deployment paths

`syslog serve mcp` runs as an HTTP MCP server because it needs persistent syslog UDP/TCP listeners:

| Path | How | Credentials |
|------|-----|-------------|
| **Plugin** | Claude Code connects via HTTP to a running instance | `${userConfig.*}` in `.mcp.json` for URL and token |
| **Docker** | `docker compose up -d` | `.env` file |
| **Bare metal** | `cargo run --release -- serve mcp` or `syslog serve mcp` | `config.toml` or env vars |

`syslog mcp` is a query-only local child process mode for stdio MCP clients. It uses `SYSLOG_MCP_DB_PATH` and logging variables, but does not require `SYSLOG_MCP_TOKEN` and does not bind network ports.

Direct CLI commands such as `syslog search`, `syslog tail`, and `syslog stats`
use the same query-only runtime and the same `SYSLOG_MCP_DB_PATH`. They are not
MCP transports and do not use `SYSLOG_MCP_TOKEN`.

## Syslog listener

| Variable | Required | Default | Description | Sensitive |
| --- | --- | --- | --- | --- |
| `SYSLOG_HOST` | no | `0.0.0.0` | Listen host for UDP+TCP syslog | no |
| `SYSLOG_PORT` | no | `1514` | Listen port (shared UDP and TCP) | no |
| `SYSLOG_MAX_MESSAGE_SIZE` | no | `8192` | Max message size in bytes | no |
| `SYSLOG_BATCH_SIZE` | no | `100` | Entries per batch flush | no |
| `SYSLOG_FLUSH_INTERVAL` | no | `500` | Batch flush interval in ms | no |
| `SYSLOG_WRITE_CHANNEL_CAPACITY` | no | `10000` | Internal parsed-message queue capacity | no |

## MCP server

| Variable | Required | Default | Description | Sensitive |
| --- | --- | --- | --- | --- |
| `SYSLOG_MCP_HOST` | no | `0.0.0.0` | HTTP bind address | no |
| `SYSLOG_MCP_PORT` | no | `3100` | HTTP listen port | no |
| `SYSLOG_MCP_TOKEN` | no | (none) | Bearer token for `/mcp`. Generate: `openssl rand -hex 32` | **yes** |
| `SYSLOG_MCP_ALLOWED_HOSTS` | no | (none) | Extra comma-separated Host header values for RMCP Host validation | no |
| `SYSLOG_MCP_ALLOWED_ORIGINS` | no | (none) | Extra comma-separated browser origins for RMCP Origin validation | no |

## Storage

| Variable | Required | Default | Description | Sensitive |
| --- | --- | --- | --- | --- |
| `SYSLOG_MCP_DB_PATH` | no | `/data/syslog.db` | SQLite database file path | no |
| `SYSLOG_MCP_POOL_SIZE` | no | `4` | Connection pool size | no |
| `SYSLOG_MCP_RETENTION_DAYS` | no | `90` | Days before automatic purge (0 = forever) | no |

## Storage budget

| Variable | Required | Default | Description | Sensitive |
| --- | --- | --- | --- | --- |
| `SYSLOG_MCP_MAX_DB_SIZE_MB` | no | `1024` | Soft DB size limit in MB (0 = disable) | no |
| `SYSLOG_MCP_RECOVERY_DB_SIZE_MB` | no | `900` | Cleanup target after DB-size breach | no |
| `SYSLOG_MCP_MIN_FREE_DISK_MB` | no | `512` | Min free disk in MB (0 = disable) | no |
| `SYSLOG_MCP_RECOVERY_FREE_DISK_MB` | no | `768` | Cleanup target after free-disk breach | no |
| `SYSLOG_MCP_CLEANUP_INTERVAL_SECS` | no | `60` | Enforcement check interval in seconds | no |
| `SYSLOG_MCP_CLEANUP_CHUNK_SIZE` | no | `2000` | Rows deleted per chunk (1 to 1,000,000) | no |

## Docker socket-proxy ingest

| Variable | Required | Default | Description | Sensitive |
| --- | --- | --- | --- | --- |
| `SYSLOG_DOCKER_INGEST_ENABLED` | no | `false` | Enable pull-based Docker log ingestion from remote docker-socket-proxy hosts | no |
| `SYSLOG_DOCKER_HOSTS` | yes, if Docker ingest is enabled | (none) | Comma-separated hostnames — each becomes `http://<host>:2375` (e.g. `squirts,tootie`) | no |
| `SYSLOG_DOCKER_RECONNECT_INITIAL_MS` | no | `1000` | Initial reconnect delay after host stream failure | no |
| `SYSLOG_DOCKER_RECONNECT_MAX_MS` | no | `30000` | Maximum reconnect delay after repeated failures | no |

Hosts specified via `SYSLOG_DOCKER_HOSTS` default to plain `http://` on port 2375 — use only on trusted private networks or behind firewall/TLS controls.

## Logging

| Variable | Required | Default | Description | Sensitive |
| --- | --- | --- | --- | --- |
| `RUST_LOG` | no | `info` | Tracing filter directive (e.g. `debug`, `syslog_mcp=trace`) | no |

## Docker / container

| Variable | Required | Default | Description | Sensitive |
| --- | --- | --- | --- | --- |
| `SYSLOG_UID` | no | `1000` | Container user ID | no |
| `SYSLOG_GID` | no | `1000` | Container group ID | no |
| `SYSLOG_MCP_CONFIG_VOLUME` | no | `./config` | Read-only config mount for optional config files | no |
| `DOCKER_NETWORK` | no | `syslog-mcp` | External Docker network name | no |

## Token generation

```bash
openssl rand -hex 32
```

Store the result in `SYSLOG_MCP_TOKEN` in your `.env` file.

## See also

- [AUTH.md](AUTH.md) -- how tokens are used for authentication
- [TRANSPORT.md](TRANSPORT.md) -- transport-specific variable usage
- [../CONFIG.md](../CONFIG.md) -- full configuration reference with validation rules
