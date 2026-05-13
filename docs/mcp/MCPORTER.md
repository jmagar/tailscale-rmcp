# Live Smoke Testing (mcporter) -- syslog-mcp

End-to-end verification against a running syslog-mcp server. Complements unit tests in [TESTS.md](TESTS.md).

## Purpose

`scripts/smoke-test.sh` exercises the full MCP server stack: auth, tool dispatch, and response validation against a live syslog-mcp instance.

## Location

```
scripts/smoke-test.sh       # Full smoke test
tests/test_live.sh          # Extended live integration tests
tests/mcporter/test-tools.sh  # mcporter-based tool tests
```

## Running

```bash
# Ensure server is running
just up

# Run smoke tests
just test-live
# or: bash scripts/smoke-test.sh
```

## mcporter configuration

mcporter config is at `config/mcporter.json`:

```json
{
  "servers": {
    "syslog-mcp": {
      "transport": "http",
      "url": "http://localhost:3100/mcp"
    }
  }
}
```

## Manual mcporter commands

```bash
# List available tools
mcporter list syslog --config config/mcporter.json

# Call actions through the single syslog tool
mcporter call --config config/mcporter.json syslog.syslog action=stats
mcporter call --config config/mcporter.json syslog.syslog action=tail n=10
mcporter call --config config/mcporter.json syslog.syslog action=search query=error limit=5
mcporter call --config config/mcporter.json syslog.syslog action=hosts
mcporter call --config config/mcporter.json syslog.syslog action=errors
mcporter call --config config/mcporter.json syslog.syslog action=status
mcporter call --config config/mcporter.json syslog.syslog action=help
```

## Test assertions

The smoke test validates:
- Health endpoint returns `{"status": "ok"}`
- The single `syslog` tool is listed
- `syslog search` returns expected `count` and `logs` fields
- `syslog tail` respects the `n` parameter
- `syslog errors` returns `summary` array
- `syslog hosts` returns `hosts` array
- `syslog correlate` returns `hosts` grouped by hostname
- `syslog stats` returns numeric fields (total_logs, total_hosts, etc.)
- `syslog status` returns DB health and runtime/OTLP observability fields
- `syslog help` returns non-empty markdown text

## Failure output

```
  PASS: health endpoint returns ok
  PASS: syslog search returns count field
  FAIL: syslog tail count should be <= 10, got 50
  ---
  30 assertions: 29 PASS, 1 FAIL
```

Exit code is non-zero if any assertion fails.

## See also

- [TESTS.md](TESTS.md) -- unit and integration tests
- [CICD.md](CICD.md) -- CI workflow configuration
