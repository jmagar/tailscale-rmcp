# Scripts Reference -- syslog-mcp

Scripts used for maintenance, hooks, and testing.

## Maintenance scripts (`scripts/`)

| Script | Purpose | Usage |
| --- | --- | --- |
| `smoke-test.sh` | Live smoke test across all 8 MCP actions | `bash scripts/smoke-test.sh` |
| `backup.sh` | WAL-safe SQLite backup using PRAGMA wal_checkpoint + .backup | `bash scripts/backup.sh` |
| `reset-db.sh` | Backup first, then destructive DB reset (stop server first) | `bash scripts/reset-db.sh` |





## Hook scripts (`hooks/scripts/`)

| Script | Purpose | Trigger |
| --- | --- | --- |
| `sync-env.sh` | Sync .env.example with server variables | Claude Code lifecycle |
| `fix-env-perms.sh` | Set .env to chmod 600 | Claude Code lifecycle |


## Test scripts (`tests/`)

| Script | Purpose | Usage |
| --- | --- | --- |
| `test_live.sh` | Extended live integration tests | `just test-live` |
| `mcporter/test-tools.sh` | mcporter-based tool tests | `bash tests/mcporter/test-tools.sh` |

## Script conventions

All bash scripts follow these patterns:
- `#!/bin/bash` shebang
- `set -euo pipefail` strict mode
- Quoted variables: `"$var"`
- Non-zero exit code on failure
- Human-readable output with PASS/FAIL indicators
- JSON output where appropriate (piped through `jq`)

## See also

- [RECIPES.md](RECIPES.md) -- Justfile recipes that invoke these scripts
- [../mcp/TESTS.md](../mcp/TESTS.md) -- testing guide
- [../mcp/MCPORTER.md](../mcp/MCPORTER.md) -- mcporter smoke testing
