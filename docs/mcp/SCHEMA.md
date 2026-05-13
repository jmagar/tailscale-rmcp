# Tool Schema Documentation -- syslog-mcp

## Overview

syslog-mcp exposes one MCP tool named `syslog`. The required `action` argument selects the operation:

- `search`
- `tail`
- `errors`
- `hosts`
- `sessions`
- `search_sessions`
- `usage_blocks`
- `project_context`
- `list_ai_tools`
- `list_ai_projects`
- `correlate`
- `stats`
- `status`
- `apps`
- `source_ips`
- `timeline`
- `patterns`
- `context`
- `get`
- `ingest_rate`
- `silent_hosts`
- `clock_skew`
- `anomalies`
- `compare`
- `compose_status`
- `compose_doctor`
- `help`

The schema is defined in `src/mcp/schemas.rs` as a `serde_json::json!()` object returned by `tool_definitions()`.

## Schema Pattern

```rust
json!({
    "name": "syslog",
    "description": "Query syslog-mcp logs with action-based subcommands...",
    "inputSchema": {
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": [
                    "search",
                    "tail",
                    "errors",
                    "hosts",
                    "sessions",
                    "search_sessions",
                    "usage_blocks",
                    "project_context",
                    "list_ai_tools",
                    "list_ai_projects",
                    "correlate",
                    "stats",
                    "status",
                    "apps",
                    "source_ips",
                    "timeline",
                    "patterns",
                    "context",
                    "get",
                    "ingest_rate",
                    "silent_hosts",
                    "clock_skew",
                    "anomalies",
                    "compare",
                    "compose_status",
                    "compose_doctor",
                    "help"
                ]
            },
            "query": { "type": "string" },
            "hostname": { "type": "string" },
            "source_ip": { "type": "string" },
            "project": {
                "type": "string",
                "description": "AI project/workspace path for session actions."
            },
            "tool": {
                "type": "string",
                "description": "AI tool name, such as claude, codex, or gemini."
            },
            "severity": {
                "type": "string",
                "enum": ["emerg", "alert", "crit", "err", "warning", "notice", "info", "debug"]
            },
            "severity_min": {
                "type": "string",
                "enum": ["emerg", "alert", "crit", "err", "warning", "notice", "info", "debug"]
            },
            "app_name": { "type": "string" },
            "facility": { "type": "string" },
            "process_id": { "type": "string" },
            "from": { "type": "string" },
            "to": { "type": "string" },
            "limit": { "type": "integer" },
            "n": { "type": "integer" },
            "reference_time": { "type": "string" },
            "window_minutes": { "type": "integer" },
            "group_by": {
                "type": "string",
                "enum": ["app_name", "hostname", "host", "severity", "sev", "app"]
            },
            "bucket": {
                "type": "string",
                "enum": ["minute", "min", "m", "hour", "h", "day", "d"]
            },
            "scan_limit": { "type": "integer" },
            "top_n": { "type": "integer" },
            "log_id": {
                "type": "integer",
                "description": "For action=context: identifies the log entry for context lookup."
            },
            "timestamp": { "type": "string" },
            "before": { "type": "integer" },
            "after": { "type": "integer" },
            "id": {
                "type": "integer",
                "description": "For action=get: identifies the record to retrieve."
            },
            "by_host": { "type": "boolean" },
            "silent_minutes": { "type": "integer" },
            "since": { "type": "string" },
            "recent_minutes": { "type": "integer" },
            "baseline_minutes": { "type": "integer" },
            "a_from": { "type": "string" },
            "a_to": { "type": "string" },
            "b_from": { "type": "string" },
            "b_to": { "type": "string" }
        },
        "required": ["action"]
    }
})
```

## Response Format

All tool responses use MCP text content blocks. The `text` field contains pretty-printed JSON:

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\"count\": 3, \"logs\": [...]}"
    }
  ]
}
```

Error responses add `"isError": true` or return an MCP invalid-params error for validation failures.

## Validation

Input validation happens in the action handlers, not only at the schema level:

- `action` is required and must be one of the supported actions
- `limit` values are capped at their action-specific maximum
- `severity` and `severity_min` are validated against known syslog levels
- `reference_time`, `from`, `to`, `timestamp`, `since`, `a_from`, `a_to`, `b_from`, and `b_to` timestamps are parsed as RFC 3339 and normalized to UTC where required
- Unknown parameters are ignored

## See Also

- [TOOLS.md](TOOLS.md) -- tool reference with parameters and response shapes
- [PATTERNS.md](PATTERNS.md) -- code patterns for tool dispatch
