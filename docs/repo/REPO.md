# Repository Layout

| Path | Purpose |
|---|---|
| `src/tailscale.rs` | Raw Tailscale REST API client. |
| `src/app.rs` | Service layer and destructive gate. |
| `src/mcp/` | MCP server, schemas, prompts, resources, metadata. |
| `src/cli.rs` | CLI parser, dotenv loading, doctor checks. |
| `src/main.rs` | Mode dispatch and auth policy selection. |
| `tests/` | Rust unit/integration tests. |
| `tests/mcporter/` | Live MCP smoke scripts. |
| `packages/tailscale-rmcp/` | npm package. |
| `plugins/tailscale/` | Claude/Codex plugin bundle. |
| `scripts/` | Release, install, version, and docs helpers. |
| `config/` | Runtime helper config, including mcporter. |
| `docs/` | Curated docs plus generated references. |

The binary target is `rtailscale`.
