# Technology Choices -- syslog-mcp

Technology stack reference and rationale.

## Language: Rust

Rust was chosen for syslog-mcp because:
- High-throughput syslog ingestion requires low-latency, zero-copy message handling
- Memory safety without garbage collection prevents the pauses that would cause UDP packet loss
- Single static binary simplifies Docker image and deployment
- `rusqlite` with bundled SQLite avoids system library version issues

## Async runtime: tokio

Full-featured async runtime for concurrent UDP/TCP listeners, batch writer, and HTTP server. The `full` feature set enables:
- `tokio::net` -- UDP/TCP socket binding
- `tokio::sync` -- mpsc channels, semaphores
- `tokio::signal` -- graceful shutdown
- `tokio::time` -- batch flush intervals, idle timeouts
- `tokio::task::spawn_blocking` -- offload synchronous SQLite calls

## HTTP framework: axum

Minimal, composable HTTP framework built on tokio and tower:
- Native tower middleware support (CORS, tracing)
- Type-safe state extraction
- Composable router with method routing
- Mounts RMCP's Tower-compatible Streamable HTTP service

## MCP SDK: rmcp

RMCP owns MCP lifecycle, Streamable HTTP framing, Host/Origin validation, tool listing, and tool calls. syslog-mcp uses stateless JSON-response mode so normal request/response calls return `Content-Type: application/json`.

## Database: SQLite (rusqlite + r2d2)

SQLite was chosen over PostgreSQL/MySQL because:
- Zero-dependency deployment (bundled, no external database server)
- WAL mode enables concurrent reads during writes
- FTS5 provides full-text search with porter stemming
- Single-file database simplifies backup and migration
- Sufficient throughput for homelab log volumes (thousands of messages/second)

| Crate | Purpose |
| --- | --- |
| `rusqlite` | SQLite driver with bundled SQLite and FTS5 vtab support |
| `r2d2` | Generic connection pooling |
| `r2d2_sqlite` | SQLite adapter for r2d2 |

## Syslog parsing: syslog_loose

Lenient syslog parser that handles both RFC 3164 (BSD) and RFC 5424 (IETF) formats. The "loose" parsing is critical for homelab environments where devices send non-compliant syslog messages (UniFi CEF, ATT router logs, etc.).

## Serialization: serde + serde_json + toml

- `serde` -- derive macros for all data structures
- `serde_json` -- tool argument/result payloads and JSON output formatting
- `toml` -- config.toml parsing

## Time: chrono

RFC 3339 timestamp parsing and formatting. Used for:
- Parsing syslog timestamps
- Time range filtering in search queries
- Correlation window calculation

## Auth: subtle

Constant-time byte comparison for bearer token validation. Prevents timing side-channel attacks.

## Filesystem: rustix

Low-level filesystem operations for free disk space measurement (`statvfs`). Used by the storage budget enforcement system.

## Logging: tracing + tracing-subscriber

Structured, span-based logging with environment filter support:
- `RUST_LOG` directive parsing
- Target-based filtering (per-module verbosity)
- Human-readable console output with timestamps

## Error handling: anyhow

Flexible error type for application code. `anyhow::Result` is used throughout for:
- Config loading errors
- Database errors
- Tool execution errors
- Propagation with `?` operator

## Development dependencies

| Crate | Purpose |
| --- | --- |
| `tempfile` | Temporary directories for isolated test databases |
| `serial_test` | Serialize tests that mutate environment variables |
| `tower` | HTTP testing utilities for axum handler tests |

## See also

- [ARCH.md](ARCH.md) -- architecture overview
- [PRE-REQS.md](PRE-REQS.md) -- tool requirements
- [../INVENTORY.md](../INVENTORY.md) -- complete dependency listing
