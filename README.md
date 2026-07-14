# tailscale-rmcp

`tailscale-rmcp` is a Rust MCP server and CLI for managing a
[Tailscale](https://tailscale.com/) tailnet through the Tailscale REST API.

It exposes one MCP tool, `tailscale`, plus the `rtailscale` CLI. Agents can
list devices, inspect routes, read API keys, ACL policy, DNS settings, and
users, authorize devices, and delete devices when the destructive gate is
explicitly enabled.

**30-second path:** set `TAILSCALE_API_KEY`, then run
`npx -y tailscale-rmcp devices --json` -> start loopback HTTP with
`TAILSCALE_MCP_HOST=127.0.0.1 npx -y tailscale-rmcp serve` -> call `tools/call`
with `{"action":"devices"}`.

**Status:** operational RMCP upstream-client server. Write-capable for device
authorization; destructive device deletion requires both server opt-in and
caller confirmation. HTTP MCP supports loopback dev mode, static bearer tokens,
and Google OAuth through `lab-auth`.

**Not for:** replacing the Tailscale admin console, bypassing Tailscale account
permissions, operating multiple unrelated tailnets from one trust boundary,
storing API keys for callers, arbitrary WireGuard control, or passing Tailscale
API keys through MCP tool arguments.

## Contents

- [Naming](#naming)
- [Capabilities And Boundaries](#capabilities-and-boundaries)
- [Install](#install)
- [Quickstart](#quickstart)
- [Client Configuration](#client-configuration)
- [Runtime Surfaces](#runtime-surfaces)
- [MCP Tool Reference](#mcp-tool-reference)
- [CLI Reference](#cli-reference)
- [Configuration](#configuration)
- [Authentication](#authentication)
- [Safety And Trust Model](#safety-and-trust-model)
- [Architecture](#architecture)
- [Distribution Contract](#distribution-contract)
- [Development](#development)
- [Verification](#verification)
- [Deployment](#deployment)
- [Troubleshooting](#troubleshooting)
- [Related Servers](#related-servers)
- [Documentation](#documentation)
- [License](#license)

## Naming

| Surface | This repo |
|---|---|
| Repository | `tailscale-rmcp` |
| Rust crate | `tailscale-rmcp` |
| Binary / CLI | `rtailscale` |
| npm package | `tailscale-rmcp` |
| npm binary aliases | `tailscale-rmcp`, `rtailscale` |
| MCP tool | `tailscale` |
| Config home | `~/.tailscale-mcp` on hosts, `/data` in containers |
| Env prefixes | `TAILSCALE_*`, `TAILSCALE_MCP_*`, `TAILSCALE_RMCP_*` for npm launcher controls |

The repo, crate, and npm package use the RMCP family name. The shipped binary is
`rtailscale` to avoid shadowing the official `tailscale` CLI.

## Capabilities And Boundaries

- List devices and inspect a single device by node ID or legacy numeric device
  ID.
- Read subnet routes, API keys, ACL policy, DNS/MagicDNS settings, and tailnet
  users.
- Authorize a device for the tailnet.
- Delete a device only when `TAILSCALE_ALLOW_DESTRUCTIVE=true` and the caller
  also passes explicit confirmation.
- Provide setup and doctor commands for local plugin/runtime checks.

| This repo owns | Tailscale owns | Explicitly out of scope |
|---|---|---|
| MCP/CLI projection, request validation, HTTP MCP auth policy, response shaping, setup checks, and destructive gates. | Tailnet state, device identities, ACL semantics, DNS behavior, API key issuance, user membership, and upstream authorization. | Replacing the admin console, storing caller credentials, multi-tailnet tenancy, arbitrary WireGuard control, policy editing beyond exposed actions, and local Tailscale daemon management. |

## Install

| Path | Command | Best for | Notes |
|---|---|---|---|
| npm / npx | `npx -y tailscale-rmcp --help` | Local MCP clients and quick trials. | Downloads the matching `rtailscale` binary from GitHub Releases. |
| Release installer | `curl -fsSL https://raw.githubusercontent.com/jmagar/tailscale-rmcp/main/scripts/install.sh \| bash` | Host installs without Node. | Installs `rtailscale` for the current Linux host. |
| Docker / Compose | `docker compose up -d` | Shared HTTP MCP deployments. | Reads `.env` and exposes container port `40040`. |
| Build from source | `cargo build --release` | Development and audits. | Produces `target/release/rtailscale`. |
| Plugin | `claude plugin install plugins/tailscale` | Claude Code local plugin setup from this checkout. | Uses the packaged setup hook, skill, and local runtime metadata. |

### npm / npx

Run the stdio MCP server or CLI without a manual binary install:

```bash
npx -y tailscale-rmcp --help
npx -y tailscale-rmcp mcp
npx -y tailscale-rmcp devices --json
```

The npm package downloads `rtailscale` during `postinstall`. Override download
behavior only when testing packaging:

| Variable | Purpose |
|---|---|
| `TAILSCALE_RMCP_SKIP_DOWNLOAD=1` | Skip postinstall binary download. |
| `TAILSCALE_RMCP_VERSION` or `TAILSCALE_RMCP_BINARY_VERSION` | Select the GitHub Release tag. |
| `TAILSCALE_RMCP_REPO` | Select the GitHub repo used for release downloads. |
| `TAILSCALE_RMCP_RELEASE_BASE_URL` | Select a custom release base URL. |

### Build From Source

```bash
git clone https://github.com/jmagar/tailscale-rmcp
cd tailscale-rmcp
cargo build --release
./target/release/rtailscale --help
```

Minimum supported Rust version: 1.86.

## Quickstart

### 1. Get A Tailscale API Key

Create an API key at
<https://login.tailscale.com/admin/settings/keys>. Use the minimum capability
needed for the actions you plan to expose.

### 2. Configure The Tailnet

```bash
export TAILSCALE_API_KEY="tskey-api-..."
export TAILSCALE_TAILNET="-"          # personal, or "example.com" for orgs
```

Every Tailscale account belongs to a tailnet. `TAILSCALE_TAILNET=-` targets the
default personal tailnet; organization tailnets usually use the org domain.

### 3. Run A Safe CLI Call

```bash
npx -y tailscale-rmcp devices --json
```

### 4. Start Loopback HTTP MCP

```bash
TAILSCALE_MCP_HOST=127.0.0.1 npx -y tailscale-rmcp serve
```

In another shell:

```bash
curl -sf http://127.0.0.1:40040/health
```

### 5. Make A First MCP Call

```bash
curl -s -X POST http://127.0.0.1:40040/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"tailscale","arguments":{"action":"devices"}}}'
```

## Client Configuration

### Claude Code Stdio

```json
{
  "mcpServers": {
    "tailscale": {
      "command": "npx",
      "args": ["-y", "tailscale-rmcp", "mcp"],
      "env": {
        "TAILSCALE_API_KEY": "tskey-api-...",
        "TAILSCALE_TAILNET": "-"
      }
    }
  }
}
```

### Claude Code HTTP

```json
{
  "mcpServers": {
    "tailscale": {
      "type": "http",
      "url": "http://127.0.0.1:40040/mcp",
      "headers": {
        "Authorization": "Bearer ${TAILSCALE_MCP_TOKEN}"
      }
    }
  }
}
```

### Codex / Labby Gateway

Register Tailscale through Labby as an HTTP upstream when sharing one
long-running server, or run it directly as stdio for local-only use.

```toml
[mcp_servers.tailscale]
command = "npx"
args = ["-y", "tailscale-rmcp", "mcp"]
```

### Generic MCP JSON

```json
{
  "command": "rtailscale",
  "args": ["mcp"],
  "env": {
    "TAILSCALE_API_KEY": "tskey-api-...",
    "TAILSCALE_TAILNET": "-"
  }
}
```

Do not put `TAILSCALE_API_KEY`, OAuth secrets, passwords, SSH keys, or upstream
bearer tokens in MCP tool arguments. Use env, config files, or the MCP client's
secret storage. MCP callers never provide credentials, tokens, keys, or secrets
as action arguments.

## Runtime Surfaces

| Surface | Status | Entry point | Purpose |
|---|---:|---|---|
| MCP stdio | Supported | `rtailscale mcp`, `npx -y tailscale-rmcp mcp` | Local child-process MCP clients. |
| MCP HTTP | Supported | `rtailscale serve`, `POST /mcp` | Streamable HTTP MCP for local or shared server deployments. |
| CLI | Supported | `rtailscale <command>` | Scriptable parity and debugging. |
| REST API | Not shipped | N/A | Tailscale already owns the REST API. |
| Web UI | Not shipped | N/A | Tailscale already owns the admin console. |

## MCP Tool Reference

One MCP tool is exposed: `tailscale`. Pass the required `action` argument to
select the operation.

### Read Actions

| Action | Description | Required params | Optional params |
|---|---|---|---|
| `devices` | List all devices in the tailnet. | none | none |
| `device` | Return one device. | `id` | none |
| `device_routes` | Return subnet routes for one device. | `id` | none |
| `keys` | List API keys in the tailnet. | none | none |
| `acl` | Return ACL policy JSON. | none | none |
| `dns` | Return DNS nameservers, search paths, and MagicDNS preferences. | none | none |
| `users` | List tailnet users. | none | none |
| `help` | Return built-in action documentation. | none | none |

### Write Actions

| Action | Description | Required params | Optional params |
|---|---|---|---|
| `authorize_device` | Approve a device for the tailnet. | `id` | none |

### Destructive Actions

| Action | Description | Required params | Optional params |
|---|---|---|---|
| `delete_device` | Permanently remove a device. | `id`, `confirm=true` | none |

Device IDs may be stable node IDs such as `n1234abc` or legacy numeric device
IDs. Use `action=devices` first to discover IDs.

## CLI Reference

The binary calls the same service layer as the MCP tool:

```bash
rtailscale devices [--json]
rtailscale device <id> [--json]
rtailscale routes <device-id> [--json]
rtailscale keys [--json]
rtailscale acl [--json]
rtailscale dns [--json]
rtailscale users [--json]
rtailscale authorize <device-id> [--json]
rtailscale delete-device <device-id> --confirm [--json]
rtailscale doctor [--json]
rtailscale setup check [--json]
rtailscale setup repair [--json]
```

All commands currently print JSON. `--json` is accepted for parity with the rest
of the RMCP family.

## Configuration

Host installs read `~/.tailscale-mcp/.env` before loading config. Containers
read `/data/.env`. Process environment overrides both.

| Variable | Default | Purpose |
|---|---|---|
| `TAILSCALE_API_KEY` | unset | Tailscale API key. |
| `TAILSCALE_TAILNET` | `-` | Tailnet: org domain or `-` for personal. |
| `TAILSCALE_ALLOW_DESTRUCTIVE` | `false` | Enable `delete_device` server-side. |
| `TAILSCALE_MCP_HOST` | `0.0.0.0` | HTTP bind host. |
| `TAILSCALE_MCP_PORT` | `40040` | HTTP bind port. |
| `TAILSCALE_MCP_SERVER_NAME` | `tailscale-rmcp` | Advertised MCP server name. |
| `TAILSCALE_MCP_NO_AUTH` | `false` | Disable auth only for loopback development. |
| `TAILSCALE_MCP_TOKEN` | unset | Static bearer token for HTTP MCP. |
| `TAILSCALE_NOAUTH` | `false` | Trust an upstream gateway to enforce auth. |
| `TAILSCALE_MCP_AUTH_MODE` | `bearer` | `bearer` or `oauth`. |
| `TAILSCALE_MCP_PUBLIC_URL` | unset | Public URL for OAuth discovery. |
| `TAILSCALE_MCP_GOOGLE_CLIENT_ID` | unset | Google OAuth client ID. |
| `TAILSCALE_MCP_GOOGLE_CLIENT_SECRET` | unset | Google OAuth client secret. |
| `TAILSCALE_MCP_AUTH_ADMIN_EMAIL` | unset | Admin email for OAuth bootstrap. |
| `TAILSCALE_MCP_ALLOWED_HOSTS` | unset | Extra accepted Host header values. |
| `TAILSCALE_MCP_ALLOWED_ORIGINS` | unset | Extra accepted CORS origins. |

## Authentication

Stdio MCP runs as a local trusted child process and does not use HTTP auth.

HTTP MCP auth policy:

| State | Condition | Behavior |
|---|---|---|
| Loopback dev | `TAILSCALE_MCP_HOST` starts with `127.` or auth is explicitly disabled on loopback | Local unauthenticated development is allowed. |
| Mounted bearer | Non-loopback with `TAILSCALE_MCP_TOKEN` | Requires `Authorization: Bearer <token>` and action scopes. |
| Mounted OAuth | `TAILSCALE_MCP_AUTH_MODE=oauth` | Uses Google OAuth/JWT through `lab-auth`. |
| Trusted gateway | `TAILSCALE_NOAUTH=true` | Assumes a reverse proxy or gateway already enforced auth. |

Non-loopback HTTP startup is rejected unless bearer auth, OAuth, or
`TAILSCALE_NOAUTH=true` is configured.

## Safety And Trust Model

- Tailscale API keys are loaded from config/env only.
- MCP callers select actions and IDs, not upstream credentials.
- `delete_device` has a two-key interlock:
  `TAILSCALE_ALLOW_DESTRUCTIVE=true` on the server and caller-provided
  `confirm=true`.
- Missing IDs and unknown actions fail before upstream calls.
- Non-loopback HTTP deployments must use bearer auth, OAuth, or a trusted
  authenticated gateway.
- This bridge does not sandbox Tailscale itself. Tailscale remains responsible
  for API permissions and tailnet state changes.

## Architecture

```text
TailscaleClient  (src/tailscale.rs)  REST transport and API error handling
       |
TailscaleService (src/app.rs)        action behavior and destructive gates
       |
MCP shim         (src/mcp.rs)        JSON args -> service -> Value
CLI shim         (src/cli.rs)        argv -> service -> stdout
```

## Distribution Contract

- `Cargo.toml`, `Cargo.lock`, `packages/tailscale-rmcp/package.json`,
  `.release-please-manifest.json`, and `server.json` must agree on the released
  version.
- GitHub Releases publish the `rtailscale` binary consumed by the npm launcher.
- The npm package name is `tailscale-rmcp`; binary aliases are
  `tailscale-rmcp` and `rtailscale`.
- Docker/OCI metadata uses `ghcr.io/jmagar/tailscale-rmcp:<version>`.
- `plugins/tailscale/.mcp.json` must launch `npx -y tailscale-rmcp mcp` so
  stdio clients start the MCP transport rather than the HTTP server.
- The root README is curated. Source of truth for action behavior and config
  defaults is `src/`, plus the package, plugin, and registry manifests.

## Development

```bash
cargo fmt --check
cargo test
cargo clippy -- -D warnings
cargo build --release
npm --prefix packages/tailscale-rmcp run check
```

## Verification

```bash
python3 /home/jmagar/workspace/soma/scripts/check-readme-guide.py README.md
npm --prefix packages/tailscale-rmcp run check
cargo check
cargo test
git diff --check
```

Runtime smoke:

```bash
TAILSCALE_API_KEY=tskey-api-... \
TAILSCALE_TAILNET=- \
rtailscale devices --json
```

HTTP smoke:

```bash
TAILSCALE_MCP_HOST=127.0.0.1 rtailscale serve
curl -sf http://127.0.0.1:40040/health
```

## Deployment

Use loopback for local development:

```bash
TAILSCALE_MCP_HOST=127.0.0.1 rtailscale serve
```

Use Docker Compose for shared HTTP deployment:

```bash
cp .env.example .env
docker compose up -d
```

When binding to a non-loopback address, configure `TAILSCALE_MCP_TOKEN`,
`TAILSCALE_MCP_AUTH_MODE=oauth`, or `TAILSCALE_NOAUTH=true` behind an
authenticated gateway.

## Troubleshooting

| Symptom | Check |
|---|---|
| `TAILSCALE_API_KEY` is missing | Set it in env or `~/.tailscale-mcp/.env`. |
| Device calls return unauthorized | Refresh the API key in Tailscale admin settings. |
| HTTP `/mcp` returns unauthorized | Set `TAILSCALE_MCP_TOKEN` and send `Authorization: Bearer <token>`. |
| Stdio client hangs or logs JSON errors | Ensure client config runs `tailscale-rmcp mcp`, not the default HTTP server mode. |
| `delete_device` is rejected | Set `TAILSCALE_ALLOW_DESTRUCTIVE=true` server-side and pass `confirm=true` after verifying the target. |
| Port conflict | Set `TAILSCALE_MCP_PORT` or stop the process already using `40040`. |

## Related Servers

- [soma](https://github.com/jmagar/soma) - RMCP runtime for provider-backed MCP servers.
- [unifi-rmcp](https://github.com/jmagar/unifi-rmcp) - UniFi controller REST API bridge.
- [unraid-rmcp](https://github.com/jmagar/unraid-rmcp) - Unraid GraphQL bridge for NAS and server management.
- [apprise-rmcp](https://github.com/jmagar/apprise-rmcp) - Apprise notification fan-out bridge for many delivery backends.
- [gotify-rmcp](https://github.com/jmagar/gotify-rmcp) - Gotify push notification bridge for sends, messages, apps, and clients.
- [arcane-rmcp](https://github.com/jmagar/arcane-rmcp) - Arcane Docker management bridge for containers and related resources.
- [yarr](https://github.com/jmagar/yarr) - Media-stack bridge for Sonarr, Radarr, Prowlarr, Plex, and related services.
- [ytdl-rmcp](https://github.com/jmagar/ytdl-rmcp) - Media download and metadata workflow server.
- [synapse-rmcp](https://github.com/jmagar/synapse-rmcp) - Local Synapse workflow server for scout and flux actions.
- [cortex](https://github.com/jmagar/cortex) - Syslog and homelab log aggregation MCP server.
- [axon](https://github.com/jmagar/axon) - RAG, crawl, scrape, extract, and semantic search project.
- [labby](https://github.com/jmagar/labby) - Homelab control plane and MCP gateway project.
- [lumen](https://github.com/jmagar/lumen) - Local semantic code search MCP server.

## Documentation

- `CLAUDE.md` is the curated local operating guide for contributors and agents.
- `docs/SETUP.md` is curated plugin/setup guidance.
- `docs/OAUTH.md` is curated OAuth setup guidance.
- `plugins/tailscale/skills/tailscale/SKILL.md` is the agent usage guide.
- `src/` is the source of truth for current actions, config defaults, auth
  behavior, and CLI parsing.
- Package, plugin, Docker, and registry manifests are curated distribution
  contracts and should be checked with the verification commands above.

## License

MIT. See [LICENSE](LICENSE).
