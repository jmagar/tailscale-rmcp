# Transport Methods Reference -- syslog-mcp

## Overview

syslog-mcp supports two first-party MCP transports:

- `syslog serve mcp`: receives syslog over UDP/TCP and exposes RMCP Streamable HTTP.
- `syslog mcp`: query-only child process mode for local stdio MCP clients.

The same `syslog` binary also includes direct non-MCP CLI commands such as
`syslog search` and `syslog stats`. Those commands use the same service layer as
the MCP tool, but they are terminal commands, not MCP transports.

`syslog mcp` is intentionally query-only so child-process clients do not accidentally start network listeners or maintenance tasks.

| Transport | Auth | Use Case | Default |
| --- | --- | --- | --- |
| RMCP Streamable HTTP, stateless JSON response via `syslog serve mcp` | Bearer token (optional) | Docker, remote servers, reverse proxy | yes |
| Direct stdio via `syslog mcp` | Local process access | Local MCP clients with direct DB access | no |
| Stateful Streamable HTTP sessions/SSE | n/a | Deferred; not enabled in this release | no |

## HTTP transport

The MCP server listens on port 3100 (configurable via `SYSLOG_MCP_PORT`).

```bash
SYSLOG_MCP_HOST=0.0.0.0
SYSLOG_MCP_PORT=3100
SYSLOG_MCP_TOKEN=your-token-here   # optional
```

### Endpoints

| Endpoint | Method | Auth | Description |
| --- | --- | --- | --- |
| `/mcp` | POST | yes (when token set) | RMCP Streamable HTTP JSON-response endpoint |
| `/mcp` | GET, DELETE | yes (when token set) | `405 Method Not Allowed` after auth succeeds, or when auth is disabled; missing/invalid bearer token returns `401` first |
| `/health` | GET | no | Health check (unauthenticated) |

### Claude Code configuration

`.claude/settings.local.json`:

```json
{
  "mcpServers": {
    "syslog-mcp": {
      "type": "http",
      "url": "http://localhost:3100/mcp",
      "headers": {
        "Authorization": "Bearer your-token-here"
      }
    }
  }
}
```

### Codex CLI configuration

`.codex/mcp.json`:

```json
{
  "mcpServers": {
    "syslog-mcp": {
      "type": "http",
      "url": "http://localhost:3100/mcp",
      "headers": {
        "Authorization": "Bearer your-token-here"
      }
    }
  }
}
```

### Gemini CLI configuration

`gemini-extension.json`:

```json
{
  "mcpServers": {
    "syslog-mcp": {
      "type": "http",
      "url": "http://localhost:3100/mcp",
      "headers": {
        "Authorization": "Bearer your-token-here"
      }
    }
  }
}
```

## Stateless mode

The production server uses `StreamableHttpServerConfig::with_stateful_mode(false)` and `with_json_response(true)`. Request/response calls return `Content-Type: application/json` instead of SSE framing. Full stateful sessions with `Mcp-Session-Id`, `GET /mcp` SSE streams, and `DELETE /mcp` session cleanup are not enabled.

Raw HTTP clients must send:

```bash
curl -s -X POST http://localhost:3100/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}'
```

## Reverse proxy Host/Origin validation

RMCP validates the `Host` header to reduce DNS rebinding risk. Loopback hosts and the configured bind host are allowed by default. Add public names or proxy authorities with:

```bash
SYSLOG_MCP_ALLOWED_HOSTS=syslog.example.com,syslog.example.com:443
SYSLOG_MCP_ALLOWED_ORIGINS=https://syslog.example.com
```

## Direct stdio transport

`syslog mcp` is a local query-only MCP server over stdin/stdout:

- It exposes the same one read-only `syslog` tool with eight actions as HTTP.
- It loads `RuntimeCore::load_query_only()`.
- It does not start UDP/TCP syslog listeners.
- It does not start the HTTP server.
- It does not run retention purge or storage-budget cleanup tasks.
- It logs to stderr only; stdout is reserved for MCP protocol messages.
- It does not require `SYSLOG_MCP_TOKEN`.

Ingestion still requires the daemon to be running somewhere. Stdio mode only queries the configured SQLite database.

```json
{
  "mcpServers": {
    "syslog-mcp": {
      "command": "/path/to/syslog",
      "args": ["mcp"],
      "env": {
        "SYSLOG_MCP_DB_PATH": "/data/syslog.db",
        "RUST_LOG": "warn"
      }
    }
  }
}
```

SQLite WAL mode supports the normal deployment shape: one daemon writing log batches while one or more query-only stdio child processes read concurrently. `syslog stats` reports current storage metrics from the database and configured thresholds; query-only stdio processes do not mutate the shared storage guard state.

## Direct non-MCP CLI

Use direct CLI commands when a human or script on the host wants to query the
database without speaking MCP:

```bash
syslog search 'error AND nginx' --limit 10
syslog tail -n 20
syslog errors --from 2026-01-01T00:00:00Z
syslog hosts
syslog correlate --reference-time 2026-01-01T12:00:00Z --window-minutes 10
syslog stats --json
```

These commands load `RuntimeCore::load_query_only()` and call the shared
`SyslogService` methods directly. They do not use `/mcp`, stdin/stdout MCP
framing, or bearer auth. See [../CLI.md](../CLI.md) for the full command
reference.

## HTTP-to-stdio bridge mode

Use `mcp-remote` instead of direct stdio when the database path is not local to the MCP host, when syslog-mcp is only available through Docker/reverse proxy, or when you want to preserve HTTP bearer auth:

```json
{
  "mcpServers": {
    "syslog-mcp": {
      "command": "npx",
      "args": ["-y", "mcp-remote", "http://localhost:3100/mcp", "--transport", "http-only"]
    }
  }
}
```

## Port assignment

| Service | Default Port | Env Var |
| --- | --- | --- |
| Syslog receiver (UDP + TCP) | 1514 | `SYSLOG_PORT` |
| MCP HTTP server | 3100 | `SYSLOG_MCP_PORT` |
| MCP stdio process | n/a | `SYSLOG_MCP_DB_PATH` |

## See also

- [AUTH.md](AUTH.md) -- bearer token setup for HTTP transport
- [../CLI.md](../CLI.md) -- direct local CLI command reference
- [ENV.md](ENV.md) -- transport-related environment variables
- [CONNECT.md](CONNECT.md) -- client connection methods
