# Configuration Reference

`tailscale-rmcp` loads `config.toml` from the working directory, then applies
environment overrides. Host installs also load `~/.tailscale-mcp/.env` before
configuration; containers load `/data/.env`.

## Config file

```toml
[tailscale]
api_key = "tskey-api-..."
tailnet = "-"
allow_destructive = false

[mcp]
host = "0.0.0.0"
port = 40040
server_name = "tailscale-rmcp"
api_token = "change-me"
no_auth = false
allowed_hosts = []
allowed_origins = []

[mcp.auth]
mode = "bearer"
public_url = "https://ts.example.com"
google_client_id = ""
google_client_secret = ""
admin_email = ""
allowed_client_redirect_uris = []
sqlite_path = "/data/auth.db"
key_path = "/data/auth-jwt.pem"
access_token_ttl_secs = 3600
refresh_token_ttl_secs = 2592000
auth_code_ttl_secs = 300
register_rpm = 10
authorize_rpm = 60
disable_static_token_with_oauth = true
```

## Tailscale settings

| Env var | Default | Secret | Purpose |
|---|---|---:|---|
| `TAILSCALE_API_KEY` | unset | yes | Tailscale REST API key. |
| `TAILSCALE_TAILNET` | `-` | no | Tailnet name or `-` for the default personal tailnet. |
| `TAILSCALE_ALLOW_DESTRUCTIVE` | `false` | no | Enables `delete_device` server-side. |

## MCP HTTP settings

| Env var | Default | Secret | Purpose |
|---|---|---:|---|
| `TAILSCALE_MCP_HOST` | `0.0.0.0` | no | HTTP bind host. |
| `TAILSCALE_MCP_PORT` | `40040` | no | HTTP bind port. |
| `TAILSCALE_MCP_SERVER_NAME` | `tailscale-rmcp` | no | Advertised MCP server name. |
| `TAILSCALE_MCP_TOKEN` | unset | yes | Static bearer token for `/mcp`. |
| `TAILSCALE_MCP_NO_AUTH` | `false` | no | Disable auth for loopback development. |
| `TAILSCALE_NOAUTH` | `false` | no | Trust an upstream gateway for auth. |
| `TAILSCALE_MCP_ALLOWED_HOSTS` | unset | no | Extra Host header allow-list entries. |
| `TAILSCALE_MCP_ALLOWED_ORIGINS` | unset | no | Extra browser CORS origins. |

Non-loopback startup is refused unless bearer auth, OAuth, or
`TAILSCALE_NOAUTH=true` is configured.

## OAuth settings

OAuth uses `lab-auth` with the `TAILSCALE_MCP` env prefix. Values can come from
`config.toml` or environment; real environment variables win.

| Env var | Secret | Purpose |
|---|---:|---|
| `TAILSCALE_MCP_AUTH_MODE=oauth` | no | Enables OAuth mode. |
| `TAILSCALE_MCP_PUBLIC_URL` | no | Public base URL for discovery and callbacks. |
| `TAILSCALE_MCP_GOOGLE_CLIENT_ID` | no | Google OAuth client ID. |
| `TAILSCALE_MCP_GOOGLE_CLIENT_SECRET` | yes | Google OAuth client secret. |
| `TAILSCALE_MCP_AUTH_ADMIN_EMAIL` | no | Bootstrap/admin email. |
| `TAILSCALE_MCP_AUTH_SQLITE_PATH` | no | Auth SQLite database path. |
| `TAILSCALE_MCP_AUTH_KEY_PATH` | yes | JWT signing key path. |
| `TAILSCALE_MCP_AUTH_ALLOWED_REDIRECT_URIS` | no | Comma-separated redirect allow-list. |
| `TAILSCALE_MCP_AUTH_ACCESS_TOKEN_TTL_SECS` | no | Access token lifetime. |
| `TAILSCALE_MCP_AUTH_REFRESH_TOKEN_TTL_SECS` | no | Refresh token lifetime. |
| `TAILSCALE_MCP_AUTH_CODE_TTL_SECS` | no | OAuth code lifetime. |

## npm launcher settings

| Env var | Purpose |
|---|---|
| `TAILSCALE_RMCP_SKIP_DOWNLOAD=1` | Skip postinstall binary download. |
| `TAILSCALE_RMCP_VERSION` or `TAILSCALE_RMCP_BINARY_VERSION` | Select the release tag. |
| `TAILSCALE_RMCP_REPO` | Override the GitHub owner/repo for downloads. |
| `TAILSCALE_RMCP_RELEASE_BASE_URL` | Override the release asset base URL. |
