# Testing Guide -- syslog-mcp

## Unit tests

```bash
cargo test
```

Shortcut: `just test`

Tests are colocated with source code in `#[cfg(test)]` modules:

| Module | Tests cover |
| --- | --- |
| `src/config.rs` | Env var overrides, defaults, validation (host format, storage budget relationships, pool size) |
| `src/db.rs` | Schema init, insert/search/tail/errors/hosts/stats, FTS5 queries, retention purge, storage budget enforcement, batch retry |
| `src/syslog.rs` | RFC 3164/5424 parsing, UniFi CEF extraction, severity mapping, facility mapping, malformed input |
| `src/mcp.rs` + `src/mcp/` | Health endpoint, auth middleware (valid/invalid/missing token, no-auth mode), RMCP tool dispatch, timestamp validation, MCP lifecycle |
| `src/main.rs` | Background interval timing |

### Running specific tests

```bash
cargo test test_search           # Run tests matching "test_search"
cargo test config::tests         # Run config module tests only
cargo test -- --nocapture        # Show println/tracing output
```

### Test database handling

Database tests use `tempfile::TempDir` for isolated SQLite instances. Each test gets a fresh database, preventing cross-test contamination. The `StorageConfig::for_test()` helper provides minimal config with pool_size=1 and WAL mode disabled.

## Live smoke tests

Live tests run against a running syslog-mcp server:

```bash
just test-live
# or: bash tests/test_live.sh
```

The smoke test (`scripts/smoke-test.sh`) exercises all `syslog` actions via mcporter.
Compose diagnostics are non-mutating and are validated only for redacted shape,
so the smoke test can pass on either Docker-backed or non-Docker deployments.

Action registry covered by live/script references: `search`, `tail`, `errors`,
`hosts`, `sessions`, `search_sessions`, `usage_blocks`, `project_context`,
`list_ai_tools`, `list_ai_projects`, `correlate`, `stats`, `status`, `apps`,
`source_ips`, `timeline`, `patterns`, `context`, `get`, `ingest_rate`,
`silent_hosts`, `clock_skew`, `anomalies`, `compare`, `compose_status`,
`compose_doctor`, `help`.

### mcporter-based testing

```bash
# List available tools
mcporter list syslog --config config/mcporter.json

# Call actions through the single syslog tool
mcporter call --config config/mcporter.json syslog.syslog action=stats
mcporter call --config config/mcporter.json syslog.syslog action=status
mcporter call --config config/mcporter.json syslog.syslog action=tail n=10
mcporter call --config config/mcporter.json syslog.syslog action=search query=error limit=5
mcporter call --config config/mcporter.json syslog.syslog action=hosts
mcporter call --config config/mcporter.json syslog.syslog action=sessions
mcporter call --config config/mcporter.json syslog.syslog action=apps
mcporter call --config config/mcporter.json syslog.syslog action=source_ips
mcporter call --config config/mcporter.json syslog.syslog action=timeline
mcporter call --config config/mcporter.json syslog.syslog action=patterns
mcporter call --config config/mcporter.json syslog.syslog action=context hostname=host timestamp=2026-01-01T00:00:00Z
mcporter call --config config/mcporter.json syslog.syslog action=get id=1
mcporter call --config config/mcporter.json syslog.syslog action=ingest_rate
mcporter call --config config/mcporter.json syslog.syslog action=silent_hosts
mcporter call --config config/mcporter.json syslog.syslog action=clock_skew
mcporter call --config config/mcporter.json syslog.syslog action=anomalies
mcporter call --config config/mcporter.json syslog.syslog action=compare a_from=2026-01-01T00:00:00Z a_to=2026-01-01T01:00:00Z b_from=2026-01-01T01:00:00Z b_to=2026-01-01T02:00:00Z
mcporter call --config config/mcporter.json syslog.syslog action=compose_status
mcporter call --config config/mcporter.json syslog.syslog action=compose_doctor
```

### curl-based testing

```bash
# Health check
curl http://localhost:3100/health

# Tail recent logs
curl -s -X POST http://localhost:3100/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"syslog","arguments":{"action":"tail","n":10}}}'

# Search
curl -s -X POST http://localhost:3100/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"syslog","arguments":{"action":"search","query":"error","limit":5}}}'

# Stats
curl -s -X POST http://localhost:3100/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"syslog","arguments":{"action":"stats"}}}'

# Status
curl -s -X POST http://localhost:3100/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"syslog","arguments":{"action":"status"}}}'
```

## Testing checklist

- [ ] **All actions return expected shape** -- syslog search, syslog tail, syslog errors, syslog hosts, syslog sessions, syslog correlate, syslog stats, syslog status, syslog help
- [ ] **AI session analytics return expected shape** -- syslog search_sessions, syslog usage_blocks, syslog project_context, syslog list_ai_tools, syslog list_ai_projects
- [ ] **Auth: valid token** -- 200 with correct Bearer token
- [ ] **Auth: invalid token** -- 401 Unauthorized
- [ ] **Auth: no token when required** -- 401 Unauthorized
- [ ] **Auth: token unset** -- `/mcp` accepts requests without `Authorization`; `/health` remains unauthenticated
- [ ] **Health endpoint** -- `GET /health` returns 200 with no auth
- [ ] **FTS5 query syntax** -- AND, OR, NOT, phrases, prefix matching
- [ ] **Time range filtering** -- from/to parameters parse ISO 8601 correctly
- [ ] **Severity filtering** -- all 8 levels work
- [ ] **Retention purge** -- logs older than retention_days are deleted
- [ ] **Storage budget** -- write blocking engages when limits are breached

## CI configuration

Tests run automatically in CI via GitHub Actions:

```yaml
# .github/workflows/ci.yml
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test
```

## Test coverage

See `tests/TEST_COVERAGE.md` for detailed coverage documentation.

## See also

- [MCPORTER.md](MCPORTER.md) -- live smoke tests with mcporter
- [CICD.md](CICD.md) -- CI workflow configuration
- [LOGS.md](LOGS.md) -- error handling patterns tested here
