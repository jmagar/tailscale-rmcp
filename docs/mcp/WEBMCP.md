# Web MCP Integration -- syslog-mcp

Browser-accessible MCP endpoints and CORS configuration.

## CORS policy

syslog-mcp allows localhost origins for the configured MCP port, plus any
origins listed in `SYSLOG_MCP_ALLOWED_ORIGINS`.

## Why restricted CORS

MCP CLI clients (mcporter, curl, Claude Code) are not browser-based and ignore CORS entirely. The restriction only prevents malicious webpages visited by a LAN user from silently exfiltrating the log database via a cross-origin browser `fetch()`.

## Browser access patterns

### Allowed

- Local web dashboards served from `localhost:3100` or `127.0.0.1:3100`
- Browser clients served from origins listed in `SYSLOG_MCP_ALLOWED_ORIGINS`
- Direct navigation to `http://localhost:3100/health`

### Blocked

- Cross-origin requests from other origins (e.g., `http://evil.example.com`)
- Requests from other local ports (e.g., `http://localhost:8080`)

### Unaffected

- All non-browser clients (curl, mcporter, Claude Code, Codex, httpie)
- Reverse proxy requests (SWAG/nginx acts as the origin)

## Customizing CORS

Set a comma-separated origin allow-list:

```bash
SYSLOG_MCP_ALLOWED_ORIGINS=https://syslog.example.com,https://logs.example.com
```

Each value must be a full browser origin URL. Add matching reverse-proxy Host
headers to `SYSLOG_MCP_ALLOWED_HOSTS` when serving through public DNS.

## See also

- [AUTH.md](AUTH.md) -- bearer token authentication (required even with CORS access)
- [TRANSPORT.md](TRANSPORT.md) -- HTTP transport details
- [../GUARDRAILS.md](../GUARDRAILS.md) -- network security patterns
