# rustscale — Agent instructions

This project is `rustscale`, a Rust MCP server that exposes the Tailscale API to AI agents. The binary is named `tailscale`.

## Repository layout

```
src/
  tailscale.rs        HTTP client for api.tailscale.com/api/v2
  app.rs              TailscaleService: business logic, destructive gate
  mcp/
    tools.rs          MCP tool dispatch shim (no logic here)
    schemas.rs        JSON Schema for the tailscale tool
    rmcp_server.rs    RMCP ServerHandler
    routes.rs         Axum router
    prompts.rs        MCP prompts
  mcp.rs              AppState, AuthPolicy, build_auth_layer
  config.rs           TailscaleConfig, McpConfig, AuthConfig, env loading
  cli.rs              CLI shim (no logic here)
  main.rs             Mode dispatch, resolve_auth_policy
  lib.rs              Public surface + testing module
tests/
  cli_parse.rs        CLI arg parsing tests
  destructive_gate.rs Destructive gate unit tests
  tool_dispatch.rs    MCP dispatch shim tests
```

## What the server does

Exposes a single MCP tool `tailscale` with action dispatch. Actions:

**Read:** `devices`, `device`, `device_routes`, `keys`, `acl`, `dns`, `users`
**Write:** `authorize_device`
**Destructive:** `delete_device` (requires `TAILSCALE_ALLOW_DESTRUCTIVE=true` AND `confirm=true`)
**Meta:** `help`

## Key constraints for agents

1. **Do not add logic to shims.** `mcp/tools.rs` and `cli.rs` parse args and call `TailscaleService`. All logic belongs in `app.rs`. All HTTP belongs in `tailscale.rs`.

2. **Destructive gate is in `app.rs` only.** Do not check `allow_destructive` or `confirm` anywhere else.

3. **Do not touch `src/`.** This project's task is documentation and test compilation. Source code is correct as-is.

4. **Tests do not require a live Tailscale account.** They use a stub API key and verify gate logic, dispatch routing, and CLI parsing in-process.

## Running tests

```bash
cargo test --no-run    # compile only
cargo test             # compile + run
```

## Environment setup for testing

No env vars needed for the test suite. The tests construct `AppState` directly from `rustscale::mcp::testing` helpers.
