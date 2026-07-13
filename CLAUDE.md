# tailscale-rmcp — Claude Code instructions

## What this project is

`tailscale-rmcp` is a Rust binary (`rtailscale`) that bridges Claude to the Tailscale REST API (`https://api.tailscale.com/api/v2`) via the Model Context Protocol. It exposes a single `tailscale` MCP tool with action dispatch.

## Module map

| File | Role |
|------|------|
| `src/tailscale.rs` | `TailscaleClient` — raw HTTP client, `Authorization: Bearer` header, one method per REST endpoint |
| `src/app.rs` | `TailscaleService` — business layer, destructive gate lives here |
| `src/mcp/tools.rs` | Thin shim: parse args, call service, return `Value`. No logic. |
| `src/mcp/schemas.rs` | MCP tool JSON Schema and `TAILSCALE_ACTIONS` slice |
| `src/mcp/rmcp_server.rs` | RMCP `ServerHandler`: tools, resources, prompts |
| `src/mcp/routes.rs` | Axum router: `/mcp`, `/health`, OAuth discovery routes |
| `src/mcp/prompts.rs` | MCP prompts |
| `src/mcp.rs` | `AppState`, `AuthPolicy`, `build_auth_layer` |
| `src/config.rs` | `Config`, `TailscaleConfig`, `McpConfig`, `AuthConfig`, env loading |
| `src/cli.rs` | CLI arg parsing and dispatch (thin shim over `TailscaleService`) |
| `src/main.rs` | Mode dispatch: HTTP server / stdio / CLI; `resolve_auth_policy` |
| `src/lib.rs` | Public API surface + `testing` module (feature-gated) |

## HARD RULE: thin shims

Neither `cli.rs` nor `mcp/tools.rs` may contain business logic. They parse their input format and delegate to `TailscaleService`. The service delegates to `TailscaleClient`. All policy (especially the destructive gate) lives in `app.rs`.

Violating this rule means the CLI and MCP tool can diverge in behavior. Do not add logic to shims.

## How to add a new action

1. **`src/tailscale.rs`** — add `pub async fn your_action(&self) -> Result<Value>` that calls `self.get_json(...)` (or posts for writes).

2. **`src/app.rs`** — add a delegating method: `pub async fn your_action(&self) -> Result<Value> { self.client.your_action().await }`. Add guard logic here if needed (not in the shim).

3. **`src/mcp/tools.rs`** — add the match arm: `"your_action" => state.service.your_action().await,`. Also add the description to `HELP_TEXT`.

4. **`src/mcp/schemas.rs`** — add `"your_action"` to the `TAILSCALE_ACTIONS` slice.

5. **`src/cli.rs`** — add the `CliCommand` variant, parse arm in `CliCommand::parse`, and dispatch arm in `run`.

For actions with parameters (like `device` with `id`), follow the `device` pattern in `tools.rs` using `string_arg` / `bool_arg` / `require_id`.

## Tailnet path pattern

`TailscaleClient` has two URL helpers:

- `tailnet_url(path)` → `https://api.tailscale.com/api/v2/tailnet/<tailnet>/<path>` — use for tailnet-scoped endpoints (devices, keys, acl, dns, users)
- `device_url(device_id, path)` → `https://api.tailscale.com/api/v2/device/<id>/<path>` — use for device-specific endpoints

## Destructive gate

The gate lives exclusively in `app.rs::TailscaleService::delete_device`. It checks:

1. `self.allow_destructive` — set at construction from `TAILSCALE_ALLOW_DESTRUCTIVE`
2. `confirm: bool` — passed by the caller

Both must be true. The shim in `tools.rs` extracts `confirm` from args and passes it through — no gate logic in the shim.

## Auth policies

`AuthPolicy` is an enum defined in `src/mcp.rs`:

- `LoopbackDev` — no auth; automatically selected when `mcp.host` starts with `127.` or `no_auth=true`
- `Mounted { auth_state: None }` — static bearer token only
- `Mounted { auth_state: Some(_) }` — full OAuth (Google + JWKS)

`resolve_auth_policy` in `main.rs` builds the policy at startup.

## Environment variables

All env vars use `TAILSCALE_` prefix. See `src/config.rs::Config::load()` for the authoritative list. Key vars:

- `TAILSCALE_API_KEY` — required
- `TAILSCALE_TAILNET` — default `-` (personal)
- `TAILSCALE_ALLOW_DESTRUCTIVE` — default `false`
- `TAILSCALE_MCP_TOKEN` — static bearer token
- `TAILSCALE_MCP_AUTH_MODE` — `bearer` (default) or `oauth`

## Build commands

```bash
cargo build --release     # produces target/release/tailscale
cargo test                # run all tests
cargo clippy -- -D warnings  # lint (must pass)
cargo fmt                 # format
```

## CLI ↔ MCP parity table

Every MCP action has a CLI equivalent. Both shims call the same `TailscaleService` method.

| Service Method | MCP Action | CLI Command |
|---|---|---|
| `service.devices()` | `tailscale(action="devices")` | `tailscale devices` |
| `service.device(id)` | `tailscale(action="device", id=...)` | `tailscale device <id>` |
| `service.device_routes(id)` | `tailscale(action="device_routes", id=...)` | `tailscale routes <device-id>` |
| `service.keys()` | `tailscale(action="keys")` | `tailscale keys` |
| `service.acl()` | `tailscale(action="acl")` | `tailscale acl` |
| `service.dns()` | `tailscale(action="dns")` | `tailscale dns` |
| `service.users()` | `tailscale(action="users")` | `tailscale users` |
| `service.authorize_device(id)` | `tailscale(action="authorize_device", id=...)` | `tailscale authorize <device-id>` |
| `service.delete_device(id, confirm)` | `tailscale(action="delete_device", id=..., confirm=true)` | `tailscale delete-device <device-id> --confirm` |
| *(meta — no service call)* | `tailscale(action="help")` | `tailscale --help` |

Parity is enforced by the thin-shim rule: both `cli.rs` and `mcp/tools.rs` call the same service methods with no logic of their own.

## Test files

| File | What it tests |
|------|---------------|
| `tests/cli_parse.rs` | CLI arg parsing — no network, no async |
| `tests/destructive_gate.rs` | Two-key interlock in `TailscaleService::delete_device` |
| `tests/tool_dispatch.rs` | MCP tool dispatch shim: help, unknown actions, missing args |

Tests use stub clients (fake API key, unreachable server). They do not require a live Tailscale account.


<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:ca08a54f -->
## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

## Session Completion

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd dolt push
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
<!-- END BEADS INTEGRATION -->
