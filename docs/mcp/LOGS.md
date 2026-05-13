# Logging and Error Handling -- syslog-mcp

## Log configuration

syslog-mcp uses the `tracing` crate with `tracing-subscriber` for structured logging.

| Env Var | Values | Default |
| --- | --- | --- |
| `RUST_LOG` | Tracing filter directives | `info` |

### Filter directive examples

```bash
RUST_LOG=info                           # Default: info level for all modules
RUST_LOG=debug                          # All modules at debug
RUST_LOG=syslog_mcp=debug              # Only syslog-mcp at debug
RUST_LOG=syslog_mcp=trace,tower_http=info  # Trace syslog-mcp, info for HTTP layer
RUST_LOG=warn                           # Quiet mode: warnings and errors only
```

## Log output

All log output goes to stdout in human-readable format with timestamps, levels, and target modules:

```
2025-01-15T14:30:00.123Z  INFO syslog_mcp::main: syslog-mcp v0.3.1
2025-01-15T14:30:00.125Z  INFO syslog_mcp::main: Configuration loaded syslog_bind=0.0.0.0:1514 mcp_bind=0.0.0.0:3100
2025-01-15T14:30:00.130Z  INFO syslog_mcp::db: Database initialized path=/data/syslog.db
2025-01-15T14:30:00.132Z  INFO syslog_mcp::syslog: Syslog listeners started bind=0.0.0.0:1514
2025-01-15T14:30:00.133Z  INFO syslog_mcp::mcp: MCP server listening bind=0.0.0.0:3100
```

### Key log events

| Event | Level | Module | Meaning |
| --- | --- | --- | --- |
| Configuration loaded | INFO | main | Startup config summary |
| Database initialized | INFO | db | Schema created/migrated |
| Syslog listeners started | INFO | syslog | UDP+TCP bound |
| MCP server listening | INFO | mcp | HTTP server ready |
| MCP tool execution started | INFO | mcp | Tool call received |
| MCP tool execution completed | INFO | mcp | Tool call finished with timing |
| Retention purge tick completed | INFO | main | Hourly log cleanup count |
| Storage budget enforcement | INFO/WARN | main | Storage threshold check |
| Backpressure applied | WARN | syslog | Write channel full |
| Backpressure lifted | INFO | syslog | Write channel cleared |
| Write channel closed | ERROR | syslog | Batch writer shutting down |
| Unauthorized MCP request rejected | WARN | mcp | Invalid or missing bearer token |

## Log location

| Context | Path |
| --- | --- |
| Local dev | stdout |
| Docker | stdout (access via `just logs` or `docker compose logs -f`) |

There is no file-based logging. Container orchestrators (Docker, Kubernetes) capture stdout logs natively.

## Error handling patterns

### MCP tool errors

Tool execution errors return MCP-formatted responses with `isError: true`:

```json
{
  "content": [{"type": "text", "text": "Tool execution failed"}],
  "isError": true
}
```

The server logs the full error with timing, then returns a sanitized message to the client.

### Database errors

SQLite errors (busy, locked, corrupt) are caught and logged:
- Transient lock errors trigger retry with exponential backoff (25ms, 100ms, 250ms)
- `busy_timeout=5000` pragma prevents most lock contention
- Persistent failures after all retries log the error and return batch to the write channel

### Syslog ingestion errors

- Oversized messages (> `max_message_size`) are dropped with a WARN log
- Invalid syslog frames are parsed best-effort (facility defaults to empty, severity to "info")
- Write channel backpressure is logged on state transitions only (not per-message) to prevent log storms
- TCP idle timeout (300s default) drops zombie connections with a WARN log

### Graceful shutdown

SIGTERM and SIGINT are handled by tokio signal handlers:
1. Log "Shutdown signal received"
2. Stop accepting new HTTP connections
3. Abort retention purge and storage enforcement tasks
4. Flush remaining batch writer entries
5. Exit cleanly

### Credential safety

- Bearer tokens are never logged at any level
- Auth failure logs include method and path but not the submitted token
- `SYSLOG_MCP_TOKEN` value is never printed in startup config summary (only `mcp_auth_enabled = true/false`)

## See also

- [DEPLOY.md](DEPLOY.md) -- Docker volume mounts
- [ENV.md](ENV.md) -- `RUST_LOG` and other env vars
- [TESTS.md](TESTS.md) -- testing error conditions
