# Direct CLI Reference -- syslog-mcp

The `syslog` binary includes direct query and deployment-lifecycle commands for
humans and shell scripts. Query commands read the configured SQLite database and
call the same shared `SyslogService` methods used by the MCP tool. Compose
lifecycle commands inspect Docker/Compose directly and do not load the SQLite
query runtime.

Direct query CLI mode does not start syslog listeners, the HTTP MCP server, the
REST API, OTLP routes, retention purge, Docker ingest, or storage-budget cleanup
tasks. Keep `syslog serve mcp` running somewhere for ingestion.

## Configuration

CLI commands use the normal config loader:

1. `config.toml` in the working directory, when present
2. `SYSLOG_*`, `SYSLOG_MCP_*`, `SYSLOG_API_*`, and `SYSLOG_DOCKER_*` environment overrides

For local query use, the important setting is:

```bash
SYSLOG_MCP_DB_PATH=/data/syslog.db
```

`SYSLOG_MCP_TOKEN` is not used by direct CLI mode because it is local database
access, not HTTP access.

## Output

All commands print compact human-readable output by default. Add `--json` to
print the exact serialized `SyslogService` response shape. MCP uses the same
shape for matching actions; REST parity applies only to commands that also have
REST endpoints.

```bash
syslog stats --json
syslog search 'error AND nginx' --limit 5 --json
```

## Commands

### `syslog search`

Search logs with optional FTS5 query and filters.

```bash
syslog search 'error AND nginx' --hostname proxy --limit 10
syslog search '"disk full"' --source-ip 10.0.0.5:514 --from 2026-01-01T00:00:00Z
```

Flags:

| Flag | Description |
| --- | --- |
| positional query | Optional SQLite FTS5 query. Multiple words are joined with spaces. |
| `--hostname HOST` | Exact claimed hostname filter |
| `--source-ip SOURCE` | Exact source identifier filter |
| `--severity LEVEL` | Syslog severity filter: `emerg`, `alert`, `crit`, `err`, `warning`, `notice`, `info`, `debug` |
| `--app-name APP` | Application/process name filter |
| `--from TIME` | RFC3339 start timestamp |
| `--to TIME` | RFC3339 end timestamp |
| `--limit N` | Maximum returned rows |
| `--json` | Print JSON response |

### `syslog tail`

Return recent log entries, optionally filtered by host, source, or app.

```bash
syslog tail -n 20
syslog tail 50 --hostname nas --app-name kernel
```

Flags:

| Flag | Description |
| --- | --- |
| positional `N` | Number of rows to return |
| `-n N`, `--n N` | Number of rows to return |
| `--hostname HOST` | Exact claimed hostname filter |
| `--source-ip SOURCE` | Exact source identifier filter |
| `--app-name APP` | Application/process name filter |
| `--json` | Print JSON response |

### `syslog errors`

Summarize error and warning counts by host and severity.

```bash
syslog errors
syslog errors --from 2026-01-01T00:00:00Z --to 2026-01-02T00:00:00Z --json
```

Flags:

| Flag | Description |
| --- | --- |
| `--from TIME` | RFC3339 start timestamp |
| `--to TIME` | RFC3339 end timestamp |
| `--json` | Print JSON response |

### `syslog hosts`

List all known hosts with log counts and last-seen timestamps.

```bash
syslog hosts
syslog hosts --json
```

### `syslog sessions`

List AI transcript sessions grouped by project.

```bash
syslog sessions --project /home/jmagar/workspace/syslog-mcp --limit 20
```

Flags:

| Flag | Description |
| --- | --- |
| `--project PATH` | Exact project path filter |
| `--tool TOOL` | AI tool filter: `claude`, `codex`, or `gemini` |
| `--hostname HOST` | Filter by host |
| `--from TIME` | RFC3339 start timestamp |
| `--to TIME` | RFC3339 end timestamp |
| `--limit N` | Maximum returned rows |
| `--json` | Print JSON response |

### `syslog ai search`

Ranked grouped session search across AI transcript rows.

```bash
syslog ai search authentication --tool claude --limit 10
```

### `syslog ai blocks`

Bucket AI activity into 5-hour UTC windows.

```bash
syslog ai blocks --project /home/jmagar/workspace/syslog-mcp
```

### `syslog ai context`

Summarize one AI project path.

```bash
syslog ai context --project /home/jmagar/workspace/syslog-mcp --limit 5
```

### `syslog ai tools`

List distinct AI tools with counts.

```bash
syslog ai tools --json
```

### `syslog ai projects`

List distinct AI projects with counts.

```bash
syslog ai projects --tool claude
```

### `syslog ai index`

Explicitly scan local transcript roots (`~/.claude/projects`, `~/.codex/sessions`) or one `--path`.

```bash
syslog ai index
syslog ai index --path ~/.claude/projects
```

### `syslog ai add`

Ingest one explicit transcript file.

```bash
syslog ai add --file ~/.claude/projects/example/session.jsonl
```

### `syslog correlate`

Find related events around a reference timestamp. Results are grouped by host.

```bash
syslog correlate --reference-time 2026-01-01T12:00:00Z --window-minutes 10
syslog correlate 2026-01-01T12:00:00Z --severity-min err --query timeout --limit 50
```

Flags:

| Flag | Description |
| --- | --- |
| positional reference time | RFC3339 center timestamp |
| `--reference-time TIME` | RFC3339 center timestamp |
| `--window-minutes N` | Minutes before and after the reference time |
| `--severity-min LEVEL` | Minimum severity to include |
| `--hostname HOST` | Exact claimed hostname filter |
| `--source-ip SOURCE` | Exact source identifier filter |
| `--query FTS` | Optional FTS5 query |
| `--limit N` | Maximum total events |
| `--json` | Print JSON response |

### `syslog stats`

Print database and storage guardrail metrics.

```bash
syslog stats
syslog stats --json
```

### `syslog compose`

Diagnose and manage the Docker Compose deployment without opening the SQLite
database.

```bash
syslog compose doctor
syslog compose status --json
syslog compose pull
syslog compose up
syslog compose restart
syslog compose logs --tail 20
syslog compose down --yes
```

Common target flags:

| Flag | Description |
| --- | --- |
| `--compose-file FILE` | Explicit Compose file |
| `--project-dir DIR` | Explicit Compose project directory |
| `--project-name NAME` | Compose project name, only safe with a file/dir or live labels |
| `--service NAME` | Compose service name, default `syslog-mcp` |
| `--container NAME` | Container name, default `syslog-mcp` |
| `--json` | Print JSON response |

Mutation flags:

| Flag | Description |
| --- | --- |
| `--dry-run` | Resolve and preflight without running Docker |
| `--allow-cwd-target` | Permit cwd `docker-compose.yml` fallback for mutation |
| `--yes` | Required for non-interactive destructive `down` |

`syslog compose` refuses ambiguous target discovery, mismatched requested
project/service selectors, cwd fallback without confirmation,
project-name-only mutations, missing Compose files, systemd owner conflicts,
non-target listeners on syslog ports, and destructive service stop without
`--yes`. `down` is intentionally service-scoped (`docker compose stop
syslog-mcp`), not a project-wide `docker compose down`.

## Relationship to MCP

The direct CLI and MCP tool share the same business layer:

| CLI command | MCP action |
| --- | --- |
| `syslog search` | `syslog` with `action="search"` |
| `syslog tail` | `syslog` with `action="tail"` |
| `syslog errors` | `syslog` with `action="errors"` |
| `syslog hosts` | `syslog` with `action="hosts"` |
| `syslog sessions` | `syslog` with `action="sessions"` |
| `syslog ai search` | `syslog` with `action="search_sessions"` |
| `syslog ai blocks` | `syslog` with `action="usage_blocks"` |
| `syslog ai context` | `syslog` with `action="project_context"` |
| `syslog ai tools` | `syslog` with `action="list_ai_tools"` |
| `syslog ai projects` | `syslog` with `action="list_ai_projects"` |
| `syslog correlate` | `syslog` with `action="correlate"` |
| `syslog stats` | `syslog` with `action="stats"` |
| `syslog compose status` | `syslog` with `action="compose_status"` (redacted read-only projection only) |
| `syslog compose doctor` | `syslog` with `action="compose_doctor"` (redacted read-only projection only) |

The MCP-only `status` and `help` actions are runtime/protocol helpers, not
direct database queries. Compose mutations (`up`, `down`, `restart`, `pull`,
`logs`) are CLI-only and are not exposed over MCP.

Use direct CLI mode for terminal queries and scripts on a host that can read the
SQLite database. Use MCP HTTP or `syslog mcp` when an MCP client needs tool
access.

## See also

- [README.md](../README.md) -- project overview and quick examples
- [mcp/TOOLS.md](mcp/TOOLS.md) -- MCP action reference
- [mcp/TRANSPORT.md](mcp/TRANSPORT.md) -- HTTP and stdio MCP transports
- [CONFIG.md](CONFIG.md) -- config file and environment reference
