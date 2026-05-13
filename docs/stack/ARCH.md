# Architecture Overview -- syslog-mcp

## Dual-port design

syslog-mcp is a dual-port server combining a syslog receiver and an MCP query interface:

```
                    ┌──────────────────────────────────────┐
  rsyslog/syslog-ng ──▶  UDP :1514 ──┐                     │
  network devices   ──▶  TCP :1514 ──┤                     │
                    │               ▼                     │
                    │     parse_syslog()                  │
                    │        │                            │
                    │        ▼                            │
                    │     mpsc channel (10K buffer)       │
                    │        │                            │
                    │        ▼                            │
                    │     batch_writer()                  │
                    │        │                            │
                    │        ▼                            │
                    │     SQLite + FTS5 (WAL mode)        │
                    │        ▲                            │
                    │        │                            │
  MCP clients ◀────────▶ RMCP HTTP :3100/mcp             │
                    │                                    │
                    │     Background tasks:               │
                    │       - Hourly retention purge      │
                    │       - Storage budget enforcement   │
                    └──────────────────────────────────────┘
```

## Request flow (MCP)

```
MCP Client (Claude Code / Codex / Gemini / curl)
    │
    ▼
HTTP Transport (axum, port 3100)
    │
    ▼
Auth Middleware (bearer token via subtle::ConstantTimeEq)
    │
    ▼
RMCP Streamable HTTP service (stateless JSON-response mode)
    │
    ▼
RMCP tool adapter (validate input, call LogService)
    │
    ▼
run_db() — spawn_blocking to avoid blocking tokio
    │
    ▼
SQLite (r2d2 pool, rusqlite, FTS5 for full-text search)
    │
    ▼
JSON Response (serde_json serialization)
```

## Data flow (syslog ingestion)

```
UDP/TCP packet arrives
    │
    ▼
parse_syslog() — syslog_loose parser, RFC 3164/5424
    │              extracts: timestamp, hostname, facility, severity, app_name, message
    │              captures: source_ip from network address
    │
    ▼
mpsc::channel — 10K entry buffer, applies backpressure when full
    │
    ▼
batch_writer() — collects entries, flushes on batch_size or flush_interval
    │              checks storage budget state before writing
    │
    ▼
insert_logs_batch() — SQLite transaction with retry (25ms, 100ms, 250ms)
    │                   FTS5 INSERT trigger fires per row
    │
    ▼
hosts table — UPSERT updates last_seen and log_count
```

## Module structure

| Module | File | Responsibility |
| --- | --- | --- |
| Entry point | `main.rs` | Tokio runtime, config load, DB init, task spawning, graceful shutdown |
| Config | `config.rs` | Three-layer config (defaults + TOML + env vars), validation |
| Database | `db.rs` | Connection pool, schema, migrations, all SQL queries, storage budget |
| Syslog | `syslog.rs` | UDP/TCP listeners, message parsing, batch writer, backpressure |
| MCP | `mcp.rs` | Axum router, auth middleware, RMCP Streamable HTTP, tool adapter |

## SQLite schema

```sql
-- Main log table
CREATE TABLE logs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp   TEXT NOT NULL,
    hostname    TEXT NOT NULL,
    facility    TEXT,
    severity    TEXT NOT NULL,
    app_name    TEXT,
    process_id  TEXT,
    message     TEXT NOT NULL,
    raw         TEXT NOT NULL,
    received_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    source_ip   TEXT NOT NULL DEFAULT ''
);

-- FTS5 full-text index
CREATE VIRTUAL TABLE logs_fts USING fts5(
    message, content='logs', content_rowid='id', tokenize='porter unicode61'
);

-- Host registry
CREATE TABLE hosts (
    hostname    TEXT PRIMARY KEY,
    first_seen  TEXT NOT NULL,
    last_seen   TEXT NOT NULL,
    log_count   INTEGER NOT NULL DEFAULT 0
);

-- Indexes
CREATE INDEX idx_logs_timestamp ON logs(timestamp);
CREATE INDEX idx_logs_hostname ON logs(hostname);
CREATE INDEX idx_logs_severity ON logs(severity);
CREATE INDEX idx_logs_app_name ON logs(app_name);
CREATE INDEX idx_logs_host_time ON logs(hostname, timestamp);
CREATE INDEX idx_logs_sev_time ON logs(severity, timestamp);
CREATE INDEX idx_logs_received_at ON logs(received_at);
CREATE INDEX idx_logs_hostname_received_at ON logs(hostname, received_at);
```

## Error handling

| Source | Error | Response |
| --- | --- | --- |
| Auth middleware | Missing/invalid token | HTTP 401, JSON error -32001 |
| Tool dispatch | Unknown tool name | RMCP method/tool error |
| Tool handler | Missing required param | RMCP invalid params error |
| Database | Query error | MCP content with `isError: true` |
| Syslog | Oversized message | Dropped with WARN log |
| Syslog | Write channel full | Backpressure applied |
| Storage | Budget exceeded | Batch writer blocked until recovery |

## Cross-references

- [TECH.md](TECH.md) -- technology stack choices
- [../mcp/TOOLS.md](../mcp/TOOLS.md) -- MCP tool definitions
- [../upstream/CLAUDE.md](../upstream/CLAUDE.md) -- upstream integration (self-contained)
