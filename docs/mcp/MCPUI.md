# MCP UI Patterns -- syslog-mcp

Protocol-level UI hints for MCP servers to improve client-side rendering of tools and results.

## Current state

syslog-mcp does not currently include MCP UI annotations in its tool schemas. Tool inputs use standard JSON Schema properties without `x-ui-*` extensions.

## Schema annotations available

Tool schemas support optional UI hints that clients can interpret:

### Parameter widgets

```json
{
  "severity": {
    "type": "string",
    "enum": ["emerg", "alert", "crit", "err", "warning", "notice", "info", "debug"],
    "description": "Filter by syslog severity level"
  }
}
```

The `enum` constraint already enables clients to render a dropdown/select widget without explicit UI annotations.

### Time range inputs

```json
{
  "from": {
    "type": "string",
    "description": "Start of time range (ISO 8601, e.g., '2025-01-15T00:00:00Z')"
  }
}
```

Clients that support datetime widgets can detect ISO 8601 format from the description.

## Response formatting

Tool responses return JSON as text content. Clients render this according to their capabilities:
- CLI clients: raw JSON or formatted with jq
- Web clients: parsed and rendered as tables
- LLM clients: interpreted directly

## Future enhancements

If MCP UI annotations are adopted:

| Action | Possible UI hint |
| --- | --- |
| `syslog search` | Multi-line text input for query, datetime pickers for from/to |
| `syslog tail` | Slider for n parameter |
| `syslog correlate` | Datetime picker for reference_time, slider for window_minutes |
| `syslog errors` | Datetime range picker |

## See also

- [TOOLS.md](TOOLS.md) -- tool reference with current schemas
- [SCHEMA.md](SCHEMA.md) -- schema documentation
