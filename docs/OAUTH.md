# OAuth Setup

HTTP MCP can use Google OAuth through `lab-auth`. Use OAuth when the server is
mounted behind a public or shared endpoint and clients should authenticate as
users instead of sharing one static bearer token.

## Required settings

```bash
TAILSCALE_MCP_AUTH_MODE=oauth
TAILSCALE_MCP_PUBLIC_URL=https://ts.example.com
TAILSCALE_MCP_GOOGLE_CLIENT_ID=...
TAILSCALE_MCP_GOOGLE_CLIENT_SECRET=...
TAILSCALE_MCP_AUTH_ADMIN_EMAIL=admin@example.com
```

Equivalent `config.toml` values under `[mcp.auth]` are also accepted. Runtime
environment variables override `config.toml`.

## Google console

Create an OAuth client in Google Cloud Console and configure redirect URIs for
the clients you expect to use. The exact redirect URL depends on the MCP client;
for Claude web/API connector flows, include the callback URI required by
Anthropic's MCP connector docs.

## Discovery routes

OAuth mode exposes:

```text
/mcp/.well-known/oauth-authorization-server
/mcp/.well-known/openid-configuration
/mcp/.well-known/oauth-protected-resource
```

The main tool endpoint remains:

```text
/mcp
```

## Storage

OAuth state is stored in SQLite and JWT keys on disk:

```bash
TAILSCALE_MCP_AUTH_SQLITE_PATH=/data/auth.db
TAILSCALE_MCP_AUTH_KEY_PATH=/data/auth-jwt.pem
```

Keep both paths on persistent storage for deployed servers.

## Static tokens in OAuth mode

`disable_static_token_with_oauth` defaults to true. Keep it enabled unless a
trusted gateway or migration path needs bearer fallback.
