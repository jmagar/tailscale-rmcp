# rustscale — Tailscale MCP Server

`rustscale` is a Rust binary named `tailscale` that bridges Claude (and any MCP client) to the [Tailscale REST API](https://api.tailscale.com/api/v2) via the Model Context Protocol.

It exposes a single MCP tool called `tailscale` with an `action` parameter. Actions cover reading device/network state, authorizing devices, and (with explicit opt-in) deleting devices.

## What is a tailnet?

Every Tailscale account belongs to a **tailnet** — the private network that connects all your devices. The tailnet identifier is either:

- `"-"` — personal accounts (default)
- `"example.com"` — organization accounts (your org's domain)

Set it once via `TAILSCALE_TAILNET`; all API paths are scoped to it automatically (`/api/v2/tailnet/<tailnet>/...`).

## Quickstart

### 1. Get a Tailscale API key

Go to <https://login.tailscale.com/admin/settings/keys> and create an API key.

### 2. Configure environment

```bash
export TAILSCALE_API_KEY="tskey-api-..."
export TAILSCALE_TAILNET="-"          # personal, or "example.com" for orgs
export TAILSCALE_MCP_TOKEN="$(openssl rand -hex 32)"
```

### 3. Run as stdio MCP server (Claude Desktop)

```bash
tailscale mcp
```

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "tailscale": {
      "command": "/path/to/tailscale",
      "args": ["mcp"],
      "env": {
        "TAILSCALE_API_KEY": "tskey-api-...",
        "TAILSCALE_TAILNET": "-"
      }
    }
  }
}
```

### 4. Run as Streamable HTTP MCP server

```bash
tailscale serve
```

The server starts on `0.0.0.0:7575`. Connect your MCP client to `http://localhost:7575/mcp` with `Authorization: Bearer <your-token>`.

## Actions

All actions are dispatched via the single `tailscale` MCP tool with an `action` string argument.

### Read

| Action | Parameters | Description |
|--------|-----------|-------------|
| `devices` | — | List all devices in the tailnet |
| `device` | `id` (required) | Single device details |
| `device_routes` | `id` (required) | Subnet routes for a device |
| `keys` | — | API keys in the tailnet |
| `acl` | — | ACL policy (JSON format) |
| `dns` | — | DNS nameservers, search paths, and MagicDNS preferences (aggregated) |
| `users` | — | Users in the tailnet |

### Write (non-destructive)

| Action | Parameters | Description |
|--------|-----------|-------------|
| `authorize_device` | `id` (required) | Approve a device for the tailnet |

### Destructive

| Action | Parameters | Description |
|--------|-----------|-------------|
| `delete_device` | `id` (required), `confirm=true` | Permanently remove a device |

Destructive actions require **both** conditions:
1. `TAILSCALE_ALLOW_DESTRUCTIVE=true` set on the server
2. `confirm=true` passed in the tool call arguments

### Meta

| Action | Description |
|--------|-------------|
| `help` | Built-in action documentation |

### Device IDs

The `id` parameter accepts either the stable **node ID** (e.g. `n1234abc`) or the legacy numeric device ID. Use `action=devices` to list all devices with their IDs.

## CLI usage

The binary also works as a direct CLI against your tailnet:

```bash
tailscale devices [--json]
tailscale device <id> [--json]
tailscale routes <device-id> [--json]
tailscale keys [--json]
tailscale acl [--json]
tailscale dns [--json]
tailscale users [--json]
tailscale authorize <device-id> [--json]
tailscale delete-device <device-id> --confirm [--json]
```

All commands print pretty-printed JSON.

## Auth modes

### Bearer token (default)

Set `TAILSCALE_MCP_TOKEN` to a static secret. MCP clients authenticate with `Authorization: Bearer <token>`.

```bash
TAILSCALE_MCP_TOKEN="$(openssl rand -hex 32)" tailscale serve
```

### OAuth (Google)

Set `TAILSCALE_MCP_AUTH_MODE=oauth`. The server runs a full OAuth 2.0 / PKCE flow and issues JWTs with scopes `tailscale:read`, `tailscale:write`, and `tailscale:admin`.

```bash
TAILSCALE_MCP_AUTH_MODE=oauth \
TAILSCALE_MCP_PUBLIC_URL=https://tailscale.example.com \
TAILSCALE_MCP_GOOGLE_CLIENT_ID=... \
TAILSCALE_MCP_GOOGLE_CLIENT_SECRET=... \
TAILSCALE_MCP_AUTH_ADMIN_EMAIL=admin@example.com \
tailscale serve
```

### Loopback / no-auth

Binding to `127.*` or setting `TAILSCALE_MCP_NO_AUTH=true` disables all auth. Suitable for local development only.

## Transports

| Mode | Command | MCP endpoint |
|------|---------|-------------|
| stdio | `tailscale mcp` | stdin / stdout |
| Streamable HTTP | `tailscale serve` | `http://<host>:<port>/mcp` |

## Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `TAILSCALE_API_KEY` | — | Tailscale API key **(required)** |
| `TAILSCALE_TAILNET` | `-` | Tailnet: org domain or `-` for personal |
| `TAILSCALE_ALLOW_DESTRUCTIVE` | `false` | Enable `delete_device` |
| `TAILSCALE_MCP_HOST` | `0.0.0.0` | Bind host |
| `TAILSCALE_MCP_PORT` | `7575` | Bind port |
| `TAILSCALE_MCP_NO_AUTH` | `false` | Disable auth (loopback only) |
| `TAILSCALE_MCP_TOKEN` | — | Static bearer token |
| `TAILSCALE_MCP_AUTH_MODE` | `bearer` | `bearer` or `oauth` |
| `TAILSCALE_MCP_PUBLIC_URL` | — | Public URL for OAuth discovery |
| `TAILSCALE_MCP_GOOGLE_CLIENT_ID` | — | Google OAuth client ID |
| `TAILSCALE_MCP_GOOGLE_CLIENT_SECRET` | — | Google OAuth client secret |
| `TAILSCALE_MCP_AUTH_ADMIN_EMAIL` | — | Admin email for OAuth |
| `TAILSCALE_MCP_ALLOWED_HOSTS` | — | Extra allowed Host headers (comma-separated) |
| `TAILSCALE_MCP_ALLOWED_ORIGINS` | — | Extra CORS origins (comma-separated) |
| `RUST_LOG` | `info` | Log filter (stderr only) |

## Build

```bash
cargo build --release          # produces target/release/tailscale
cargo test                     # run test suite
cargo clippy -- -D warnings    # lint
cargo fmt                      # format
```

## Destructive operation safety

`delete_device` has a two-key interlock:

1. The **server** must have `TAILSCALE_ALLOW_DESTRUCTIVE=true`. This is off by default.
2. The **caller** must pass `confirm=true` explicitly in the tool arguments.

Both must be true; either alone is not sufficient. This prevents both misconfigured-server accidents and LLM hallucination-driven deletions.

## License

MIT
