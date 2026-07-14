# Repo Inventory

| Path | Purpose |
|---|---|
| `src/tailscale.rs` | Tailscale REST client and upstream API errors. |
| `src/app.rs` | Service layer and destructive gate. |
| `src/mcp/tools.rs` | MCP argument parsing and action dispatch. |
| `src/mcp/rmcp_server.rs` | RMCP handler, auth checks, result envelopes, resources, prompts. |
| `src/mcp/schemas.rs` | Tool schema, output schema, annotations, execution, and tool `_meta`. |
| `src/mcp/metadata.rs` | Shared MCP icon and `_meta` helpers. |
| `src/mcp/routes.rs` | Axum router, `/mcp`, `/health`, and OAuth discovery routes. |
| `src/mcp/prompts.rs` | Prompt definitions. |
| `src/config.rs` | Config structs, defaults, env loading. |
| `src/cli.rs` | CLI parsing, dotenv loading, doctor checks. |
| `src/setup.rs` | Plugin setup checks and repair. |
| `packages/tailscale-rmcp/` | npm launcher package. |
| `plugins/tailscale/` | Claude/Codex plugin bundle. |
| `server.json` | MCP Registry server manifest. |
| `config/mcporter.json` | Live mcporter config for `https://ts.tootie.tv/mcp`. |
| `tests/` | Unit and integration smoke tests. |
| `docs/references/` | Generated upstream reference snapshots. |
