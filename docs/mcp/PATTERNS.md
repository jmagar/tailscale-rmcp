# Common MCP Code Patterns -- syslog-mcp

Reusable patterns in the syslog-mcp implementation.

## Action-Based Tool Dispatch

syslog-mcp exposes one public MCP tool, `syslog`, and dispatches on the required
`action` argument:

```rust
async fn execute_tool(state: &AppState, name: &str, args: Value) -> anyhow::Result<Value> {
    match name {
        "syslog" => tool_syslog(state, args).await,
        _ => Err(anyhow::anyhow!("Unknown tool: {name}")),
    }
}

async fn tool_syslog(state: &AppState, args: Value) -> anyhow::Result<Value> {
    match string_arg(&args, "action").as_deref() {
        Some("search") => tool_search_logs(state, args).await,
        Some("tail") => tool_tail_logs(state, args).await,
        Some("errors") => tool_get_errors(state, args).await,
        Some("hosts") => tool_list_hosts(state, args).await,
        Some("correlate") => tool_correlate_events(state, args).await,
        Some("stats") => tool_get_stats(state, args).await,
        Some("status") => tool_get_status(state, args).await,
        Some("help") => tool_syslog_help().await,
        _ => Err(anyhow::anyhow!("action is required")),
    }
}
```

This keeps the client-facing tool list compact while preserving separate private
handlers for each action.

## Shared SyslogService boundary

MCP tools are adapters over the shared application layer in `src/app/`. Transport code extracts JSON arguments, calls `SyslogService`, and serializes typed responses back into MCP content envelopes:

```rust
async fn tool_search_logs(state: &AppState, args: Value) -> anyhow::Result<Value> {
    let response = state
        .service
        .search_logs(SearchLogsRequest {
            query: string_arg(&args, "query"),
            hostname: string_arg(&args, "hostname"),
            source_ip: string_arg(&args, "source_ip"),
            // ...
        })
        .await?;
    Ok(serde_json::to_value(response)?)
}
```

`SyslogService` owns timestamp normalization, defaults, severity threshold expansion, correlation grouping, and bounded blocking DB execution. MCP should not call `DbPool` directly for log use cases.

## Batch writer

Syslog messages flow through an mpsc channel to a batched writer:

```
UDP/TCP listener -> parse_syslog() -> mpsc::channel -> batch_writer() -> insert_logs_batch()
```

The batch writer collects entries and flushes when either:
- The batch reaches `batch_size` entries (default 100)
- The `flush_interval` timer fires (default 500ms)

This amortizes SQLite transaction overhead across many inserts.

## Storage budget with hysteresis

The storage enforcement uses a two-threshold system:

1. **Trigger**: DB size > `max_db_size_mb` or free disk < `min_free_disk_mb`
2. **Recovery**: Delete oldest logs until DB size < `recovery_db_size_mb` and free disk > `recovery_free_disk_mb`
3. **Write block**: If cleanup cannot recover, block the batch writer

The gap between trigger and recovery thresholds prevents oscillation (delete-write-delete cycles).

## Constant-time auth

Bearer token comparison uses the `subtle` crate:

```rust
let authorized = match provided {
    Some(token) => token.as_bytes().ct_eq(expected.as_bytes()).unwrap_u8() == 1,
    None => false,
};
```

This prevents timing side-channel attacks against the bearer token.

## Backpressure logging

State-transition logging prevents log storms under load:

```rust
let at_capacity = tx.capacity() == 0;
if at_capacity && !backpressure {
    warn!("syslog write channel full - backpressure applied");
    backpressure = true;
} else if !at_capacity && backpressure {
    info!("syslog write channel cleared - backpressure lifted");
    backpressure = false;
}
```

Only the transitions are logged, not every message during backpressure.

## TCP connection limiting

A semaphore caps concurrent TCP connections:

```rust
let sem = Arc::new(Semaphore::new(max_connections));
// ...
match Arc::clone(&sem).try_acquire_owned() {
    Ok(permit) => { /* handle connection, permit drops on close */ }
    Err(_) => { /* reject connection */ }
}
```

Rejection logging is rate-limited to once per 10 seconds.

## SQLite retry with backoff

Transient SQLite lock errors trigger retry:

```rust
const RETRY_DELAYS_MS: &[u64] = &[25, 100, 250];
```

Three attempts with increasing delay before surfacing the error.

## See also

- [TOOLS.md](TOOLS.md) -- tool definitions
- [SCHEMA.md](SCHEMA.md) -- schema patterns
- [../stack/ARCH.md](../stack/ARCH.md) -- architecture overview
