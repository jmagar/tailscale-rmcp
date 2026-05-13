# Connect to MCP -- syslog-mcp

How to connect to the syslog-mcp server from every supported client.

## Via plugin (simplest)

```bash
# Claude Code
/plugin marketplace add jmagar/claude-homelab
/plugin install syslog-mcp @jmagar-claude-homelab
```

The plugin manifest handles transport and tool registration. Configure the MCP URL and optional API token when prompted.

syslog-mcp uses RMCP Streamable HTTP in stateless JSON-response mode for daemon deployments. Local stdio clients can launch `syslog mcp` when they can read the SQLite database directly.

## Claude Code CLI

```bash
claude mcp add --transport http syslog-mcp http://localhost:3100/mcp
```

With bearer auth:

```bash
claude mcp add --transport http \
  --header "Authorization: Bearer $SYSLOG_MCP_TOKEN" \
  syslog-mcp http://localhost:3100/mcp
```

### Scopes

| Flag | Scope | Config file |
| --- | --- | --- |
| `--scope project` | Current project only | `.claude/settings.local.json` |
| `--scope user` | All projects (local) | `~/.claude/settings.json` |
| (none) | Defaults to project | `.claude/settings.local.json` |

## Codex CLI

`.codex/mcp.json` (project) or `~/.codex/mcp.json` (global):

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

## Gemini CLI

`gemini-extension.json` (project root) or `~/.gemini/gemini-extension.json` (global):

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

## Direct stdio clients

Use `syslog mcp` for local query-only access. It exposes the same one read-only
`syslog` tool with eight actions as HTTP, but it does not receive syslog, start
`/mcp`, run cleanup jobs, or require `SYSLOG_MCP_TOKEN`.

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

The daemon must still be running somewhere to ingest logs into that database.

## stdio bridge to HTTP

Use an HTTP bridge when the DB path is not local to the MCP client, or when the server is remote/Docker-only:

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

## Manual configuration reference

All clients use the same `mcpServers` JSON structure. The only difference is the file path.

### Config file locations

| Client | Scope | File |
| --- | --- | --- |
| Claude Code | Project | `.claude/settings.local.json` |
| Claude Code | User | `~/.claude/settings.json` |
| Codex CLI | Project | `.codex/mcp.json` |
| Codex CLI | User | `~/.codex/mcp.json` |
| Gemini CLI | Project | `gemini-extension.json` |
| Gemini CLI | Global | `~/.gemini/gemini-extension.json` |

## Via SWAG reverse proxy

When syslog-mcp is behind SWAG, the MCP endpoint becomes:

```
https://syslog-mcp.tootie.tv/mcp
```

Configure clients to use this URL instead of `localhost:3100`.

## Verifying connection

```bash
# Health check (unauthenticated)
curl -s http://localhost:3100/health
# Expected: {"status":"ok"}

# List available tools
curl -s -X POST http://localhost:3100/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}'

# Test a tool call
curl -s -X POST http://localhost:3100/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"syslog","arguments":{"action":"stats"}}}'
```

If connection fails, check:

1. Server is running (`just up` or `just dev`)
2. Port 3100 is not blocked by firewall
3. Bearer token matches between client config and server `.env`
4. Docker port mapping is correct: `docker port syslog-mcp`

## See also

- [AUTH.md](AUTH.md) -- bearer token setup
- [ENV.md](ENV.md) -- environment variables
- [TRANSPORT.md](TRANSPORT.md) -- transport details
