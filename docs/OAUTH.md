# OAuth Authentication

syslog-mcp supports Google OAuth 2.0 as an alternative to the static bearer token. Both modes leave `/health` unauthenticated and honour the same scope-based tool dispatch.

---

## Architecture

```
                         ┌────────────────────────────────────┐
Client (browser/Claude)  │  syslog-mcp HTTP :3100             │
                         │                                    │
  GET /auth/google/... ──▶  OAuth router (bearer_only_router) │
  POST /token         ──▶  RS256 JWT issuance (lab-auth)     │
                         │                                    │
  POST /mcp           ──▶  AuthLayer (lab-auth middleware)    │
    Bearer JWT        ──▶    RS256 verify → AuthContext       │
    Bearer static     ──▶    constant-time compare            │
                         │                                    │
                         │  RMCP tool dispatch                │
                         │    scope check (syslog:read)       │
                         │    → SyslogService / SQLite        │
                         └────────────────────────────────────┘

OAuth discovery endpoints (mounted when AUTH_MODE=oauth):
  GET /.well-known/oauth-authorization-server
  GET /.well-known/oauth-protected-resource
  GET /jwks
  GET /authorize
  GET /auth/google/callback
  POST /token

NOT mounted in any mode:
  POST /register   (dynamic client registration — disabled)
  GET  /auth/login (browser HTML login page — headless server)
```

### Auth flow (authorization_code grant)

1. Client sends unauthenticated request to `/mcp` → receives `401 WWW-Authenticate: Bearer resource_metadata="…"`.
2. Client fetches `/.well-known/oauth-protected-resource` to discover the authorization server.
3. Client fetches `/.well-known/oauth-authorization-server` for the full metadata document.
4. Client constructs an `/authorize` URL (PKCE S256, `scope=syslog:read`), opens in browser.
5. User authenticates with Google; Google redirects to `/auth/google/callback`.
6. Server validates email against allowlist, issues an RS256 access token (1h TTL) and a refresh token (8h TTL).
7. Client uses `POST /token?grant_type=refresh_token` to obtain new access tokens without re-prompting.

---

## Google Console setup

1. Go to [Google Cloud Console](https://console.cloud.google.com) → **APIs & Services** → **Credentials**.
2. Click **Create Credentials** → **OAuth client ID** → **Web application**.
3. Add an authorized redirect URI: `https://YOUR_PUBLIC_URL/auth/google/callback`.
4. Copy the **Client ID** and **Client Secret**.

---

## Configuration

### Environment variables

| Variable | Required | Description |
|----------|----------|-------------|
| `SYSLOG_MCP_AUTH_MODE` | yes | Set to `oauth` to activate |
| `SYSLOG_MCP_PUBLIC_URL` | yes | Base URL (e.g. `https://syslog.example.com`). Sets issuer + audience. |
| `SYSLOG_MCP_GOOGLE_CLIENT_ID` | yes | From Google Console |
| `SYSLOG_MCP_GOOGLE_CLIENT_SECRET` | yes | From Google Console |
| `SYSLOG_MCP_AUTH_ADMIN_EMAIL` | yes | Bootstrap allowed Google account |
| `SYSLOG_MCP_AUTH_ALLOWED_REDIRECT_URIS` | no | Comma-separated non-loopback OAuth client callbacks, such as a Codex callback URL |
| `SYSLOG_MCP_AUTH_DISABLE_STATIC_TOKEN_WITH_OAUTH` | no | Defaults to `true`; set `false` to keep `SYSLOG_MCP_TOKEN` working while OAuth is active |

### config.toml `[mcp.auth]` fields

These are **not** env vars — they go in `config.toml`:

```toml
[mcp.auth]
mode = "oauth"
public_url = "https://syslog.example.com"
google_client_id = "..."         # overridden by SYSLOG_MCP_GOOGLE_CLIENT_ID
google_client_secret = "..."     # overridden by SYSLOG_MCP_GOOGLE_CLIENT_SECRET

# Single admin email (bootstrap allowlist)
admin_email = "you@example.com"

# Additional allowed emails
allowed_emails = ["colleague@example.com"]

# File paths (relative to the syslog DB directory)
sqlite_path = "auth.db"
key_path = "auth-jwt.pem"

# Token TTLs
access_token_ttl_secs = 3600    # 1h (default)
refresh_token_ttl_secs = 28800  # 8h (default; lab-auth default is 30d)

# Set false to keep static SYSLOG_MCP_TOKEN as break-glass when OAuth is active
disable_static_token_with_oauth = true   # default: true
```

---

## Gotchas

- **Refresh token TTL is 8h**, not lab-auth's default of 30d. This suits the read-only homelab profile. Adjust via `[mcp.auth].refresh_token_ttl_secs`.
- **Allowlist is required**. Without `admin_email` or `allowed_emails`, startup fails with a config error — any Google account would gain access otherwise.
- **`disable_static_token_with_oauth` defaults to `true`**. When OAuth is active, `SYSLOG_MCP_TOKEN` is rejected by default. Set `SYSLOG_MCP_AUTH_DISABLE_STATIC_TOKEN_WITH_OAUTH=false` or `disable_static_token_with_oauth = false` in config.toml for break-glass bearer access.
- **Stdio mode always uses LoopbackDev**. `cargo run -- mcp` ignores the auth config entirely — no credentials are needed or enforced.
- **Docker bind-mount ownership**. `auth.db` and `auth-jwt.pem` are written by the container UID. Host-side backup scripts may need `sudo` or a sidecar copy step.
- **`/register` and `/auth/login` are never mounted**. syslog-mcp uses the headless (`bearer_only_router`) subset of lab-auth's OAuth routes — no browser login page, no dynamic client registration.
- **RFC 9700 refresh-token rotation** is not yet implemented. The same refresh token is returned on each `POST /token?grant_type=refresh_token` call. This is tracked as known debt in CHANGELOG.md.

---

## Operator FAQ

**How do I revoke a user's access?**

Delete their row from `auth.db`:

```sql
-- Connect to auth.db (WAL-safe: stop the server first or use .backup)
DELETE FROM refresh_tokens WHERE sub = 'user@example.com';
DELETE FROM allowed_users  WHERE email = 'user@example.com';
```

Remove them from `allowed_emails` in config.toml and restart. Future authorization attempts will fail at the callback.

**How do I rotate the JWT signing key?**

```bash
# Stop the server, replace the key file, restart
docker compose down
rm /data/auth-jwt.pem   # or the path from key_path in config.toml
docker compose up -d    # server generates a new key on first boot
```

All existing access tokens become invalid immediately (they reference the old `kid`). Users must re-authenticate. Refresh tokens in the DB are also invalidated because new tokens issued with the new key will not verify against old JWTs held by clients.

**How do I add a new allowed user without restarting?**

Add their email to the `allowed_emails` list in config.toml and send `SIGHUP` (or restart). The email allowlist is checked at callback time — no DB entry is needed in advance.

**How do I check which emails are currently allowed?**

```sql
SELECT email, created_at FROM allowed_users ORDER BY created_at;
```

---

## Runtime model

The auth middleware (`lab_auth::AuthLayer`) runs on every `/mcp` request:

- **Static token**: constant-time string compare — O(1), no DB access.
- **JWT**: stateless RS256 verify — ~250µs per request, no DB access, no I/O.
- **JWKS fetch**: bounded 5s timeout; result cached in `AuthState`; no per-request fetch.

The tokio runtime is shared between the auth middleware, RMCP handler, syslog ingest, and DB writer. Auth does not write to any DB in the hot path. Under auth burst, the bottleneck is the RSA verify, not the database.
