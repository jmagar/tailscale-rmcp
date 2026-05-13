# Component Inventory -- syslog-mcp

Complete listing of all plugin components.

## MCP tools

syslog-mcp exposes one read-only MCP tool named `syslog`. The required `action`
argument selects the operation.

| Action | Description | Destructive |
| --- | --- | --- |
| `search` | Full-text search across syslog messages with FTS5 syntax, host/source_ip/severity/app/time filters | no |
| `tail` | Get N most recent log entries, optionally filtered by host, source_ip, and/or application | no |
| `errors` | Error/warning summary grouped by hostname and severity level with counts | no |
| `hosts` | List all hosts with first/last seen timestamps and total log counts | no |
| `correlate` | Cross-host event correlation within a time window around a reference timestamp | no |
| `stats` | Database statistics: total logs, hosts, time range, DB size, free disk, write-block status | no |
| `status` | Lightweight runtime status: DB health, queue/backpressure state, listener/writer counters, OTLP counters | no |
| `help` | Returns markdown documentation for all actions | no |

All MCP actions are read-only. syslog-mcp exposes no destructive operations via MCP.

## Direct CLI commands

The `syslog` binary also exposes direct local commands backed by the same service
methods as the MCP actions.

| Command | Matches MCP action | Description |
| --- | --- | --- |
| `syslog search` | `search` | Full-text search with filters |
| `syslog tail` | `tail` | Recent log entries |
| `syslog errors` | `errors` | Error/warning summary |
| `syslog hosts` | `hosts` | Known host list |
| `syslog correlate` | `correlate` | Cross-host event correlation |
| `syslog stats` | `stats` | Database and storage metrics |

## MCP resources

| URI | Description | MIME type |
| --- | --- | --- |
| `syslog://schema/mcp-tool` | JSON schema for the `syslog` MCP tool and action-based parameters | `application/json` |

## Environment variables

| Variable | Required | Default | Sensitive |
| --- | --- | --- | --- |
| `SYSLOG_HOST` | no | `0.0.0.0` | no |
| `SYSLOG_PORT` | no | `1514` | no |
| `SYSLOG_MAX_MESSAGE_SIZE` | no | `8192` | no |
| `SYSLOG_BATCH_SIZE` | no | `100` | no |
| `SYSLOG_FLUSH_INTERVAL` | no | `500` | no |
| `SYSLOG_MCP_HOST` | no | `0.0.0.0` | no |
| `SYSLOG_MCP_PORT` | no | `3100` | no |
| `SYSLOG_MCP_TOKEN` | no | (none) | yes |
| `SYSLOG_MCP_ALLOWED_HOSTS` | no | (none) | no |
| `SYSLOG_MCP_ALLOWED_ORIGINS` | no | (none) | no |
| `SYSLOG_API_ENABLED` | no | `false` | no |
| `SYSLOG_API_TOKEN` | yes when enabled | (none) | yes |
| `SYSLOG_MCP_DB_PATH` | no | `/data/syslog.db` | no |
| `SYSLOG_MCP_POOL_SIZE` | no | `4` | no |
| `SYSLOG_MCP_RETENTION_DAYS` | no | `90` | no |
| `SYSLOG_MCP_MAX_DB_SIZE_MB` | no | `1024` | no |
| `SYSLOG_MCP_RECOVERY_DB_SIZE_MB` | no | `900` | no |
| `SYSLOG_MCP_MIN_FREE_DISK_MB` | no | `512` | no |
| `SYSLOG_MCP_RECOVERY_FREE_DISK_MB` | no | `768` | no |
| `SYSLOG_MCP_CLEANUP_INTERVAL_SECS` | no | `60` | no |
| `SYSLOG_MCP_CLEANUP_CHUNK_SIZE` | no | `2000` | no |
| `RUST_LOG` | no | `info` | no |

## Plugin surfaces

| Surface | Present | Path |
| --- | --- | --- |
| Skills | yes | `skills/syslog/SKILL.md` |
| Agents | no | -- |
| Commands | no | -- |
| Hooks | yes | `hooks/` |
| Channels | no | -- |
| Output styles | no | -- |
| Schedules | no | -- |

## Network ports

| Port | Protocol | Purpose |
| --- | --- | --- |
| 1514 | UDP + TCP | Syslog receiver (RFC 3164/5424) |
| 3100 | TCP | RMCP Streamable HTTP endpoint |

## HTTP endpoints

| Endpoint | Method | Auth required | Description |
| --- | --- | --- | --- |
| `/mcp` | POST | yes (when token set) | RMCP stateless Streamable HTTP endpoint |
| `/mcp` | GET, DELETE | yes (when token set) | 401 first if token auth is enabled and the bearer token is missing/invalid; otherwise 405 in stateless mode |
| `/health` | GET | no | Health check -- verifies DB connectivity |
| `/api/search` | GET | yes when API enabled | Plain JSON log search |
| `/api/tail` | GET | yes when API enabled | Plain JSON recent logs |
| `/api/errors` | GET | yes when API enabled | Plain JSON error summary |
| `/api/hosts` | GET | yes when API enabled | Plain JSON host list |
| `/api/correlate` | GET | yes when API enabled | Plain JSON event correlation |
| `/api/stats` | GET | yes when API enabled | Plain JSON database stats |

## Docker

| Component | Value |
| --- | --- |
| Image | `ghcr.io/jmagar/syslog-mcp:latest` |
| Syslog port | `1514/udp`, `1514/tcp` |
| MCP port | `3100/tcp` |
| Health endpoint | `GET /health` (unauthenticated) |
| Compose file | `docker-compose.yml` |
| Entrypoint | `entrypoint.sh` |
| User | `1000:1000` |
| Data volume | `/data` (SQLite database) |

## CI/CD workflows

| Workflow | Trigger | Purpose |
| --- | --- | --- |
| `ci.yml` | push, PR | Lint (clippy), check, test |
| `docker-publish.yml` | tag push | Build and publish Docker image to GHCR |
| `publish-crates.yml` | tag push | Publish to crates.io |
| `codex-plugin-scanner.yml` | PR | Validate Codex plugin manifest |

## Scripts

| Script | Purpose |
| --- | --- |
| `scripts/smoke-test.sh` | Live smoke test -- all 8 MCP actions via mcporter |
| `scripts/backup.sh` | WAL-safe SQLite backup (checkpoint + `.backup` method) |
| `scripts/reset-db.sh` | WAL-safe backup + destructive DB reset for dev recovery |





## Dependencies

### Runtime

| Crate | Purpose |
| --- | --- |
| `tokio` | Async runtime (full features) |
| `axum` | HTTP framework for MCP server |
| `tower-http` | CORS and tracing middleware |
| `rusqlite` | SQLite driver (bundled, with FTS5) |
| `r2d2` / `r2d2_sqlite` | Connection pooling |
| `syslog_loose` | RFC 3164/5424 syslog parsing |
| `serde` / `serde_json` | Serialization |
| `chrono` | Timestamps |
| `toml` | Config file parsing |
| `tracing` / `tracing-subscriber` | Structured logging |
| `anyhow` | Error handling |
| `subtle` | Constant-time token comparison |
| `rustix` | Filesystem stats (free disk space) |

### Development

| Crate | Purpose |
| --- | --- |
| `tempfile` | Temporary directories for test databases |
| `serial_test` | Serialized test execution for env var tests |
| `tower` | HTTP testing utilities |
