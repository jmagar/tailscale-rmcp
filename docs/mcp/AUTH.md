# MCP Auth

Stdio MCP is local and trusted. HTTP MCP uses `AuthPolicy` from `src/mcp.rs`.

| Policy | When selected | Behavior |
|---|---|---|
| `LoopbackDev` | Host starts with `127.` or `no_auth=true` | No HTTP auth; intended for local use. |
| `Mounted { auth_state: None }` | Non-loopback bearer mode | Requires static bearer token. |
| `Mounted { auth_state: Some(_) }` | OAuth mode | Uses Google OAuth/JWT via `lab-auth`. |

Non-loopback startup is rejected unless bearer auth, OAuth, or
`TAILSCALE_NOAUTH=true` is configured.

Scopes:

- `tailscale:read` for read actions
- `tailscale:write` for write actions
- `tailscale:admin` satisfies both

The service layer still enforces its own destructive-action gate.
