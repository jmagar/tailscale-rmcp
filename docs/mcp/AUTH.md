# Authentication Reference -- syslog-mcp

## Overview

syslog-mcp has a single authentication boundary: MCP clients authenticating to the MCP HTTP server. There is no outbound authentication -- syslog-mcp is a self-contained syslog receiver with no upstream API dependency.

## Bearer token

When `SYSLOG_MCP_TOKEN` is set, all requests to `/mcp` require:

```
Authorization: Bearer {SYSLOG_MCP_TOKEN}
```

Generate a token:

```bash
openssl rand -hex 32
```

Set it in `.env`:

```bash
SYSLOG_MCP_TOKEN=<generated-token>
```

`SYSLOG_MCP_API_TOKEN` is still accepted as a deprecated compatibility alias when `SYSLOG_MCP_TOKEN` is unset.

## Authentication middleware

The `require_auth` middleware in `src/mcp/routes.rs` validates inbound tokens:

```
Request -> require_auth middleware -> Route Handler
               |
               v (401)
         Missing/invalid token
```

- Returns HTTP 401 with a JSON-RPC error envelope if the token is missing or incorrect:

```json
{"jsonrpc":"2.0","id":null,"error":{"code":-32001,"message":"unauthorized"}}
```

- Uses `subtle::ConstantTimeEq` for token comparison to prevent timing side-channel attacks
- Applies to `/mcp` RMCP requests. `/health` is outside MCP auth.

## Unauthenticated endpoints

| Endpoint | Method | Purpose |
| --- | --- | --- |
| `/health` | GET | Health check -- verifies SQLite connectivity, returns `{"status": "ok"}` |

The health endpoint is intentionally unauthenticated so Docker HEALTHCHECK, docker-compose probes, and SWAG liveness checks can reach it without credentials.

## No-auth mode

When `SYSLOG_MCP_TOKEN` is not set (the default), the MCP endpoint passes through without authentication. This is acceptable for:
- LAN-only deployments behind a firewall
- Deployments behind a reverse proxy that handles its own auth (SWAG with Authelia, Cloudflare Access)

When exposed to the internet or untrusted networks, always set `SYSLOG_MCP_TOKEN`.

## Plugin userConfig integration

When installed as a Claude Code plugin, the token is managed via `userConfig` in `.claude-plugin/plugin.json`:

```json
{
  "userConfig": {
    "syslog_mcp_token": {
      "type": "string",
      "title": "API Token",
      "description": "Bearer token for authenticating MCP requests (leave empty if auth is disabled)",
      "sensitive": true
    }
  }
}
```

Fields marked `"sensitive": true` are stored encrypted by Claude Code.

## Security practices

- Token comparison is constant-time (`subtle::ConstantTimeEq`) to prevent timing attacks
- Auth failure logs include HTTP method and path but never the submitted token value
- No tokens are logged at any log level, including `RUST_LOG=trace`
- Rotate credentials by updating `.env` and running `just restart`

## See also

- [ENV.md](ENV.md) -- environment variable reference
- [TRANSPORT.md](TRANSPORT.md) -- transport-specific auth behavior
- [../GUARDRAILS.md](../GUARDRAILS.md) -- full security guardrails
