# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-05-13

### Added

- `TailscaleClient` — raw HTTP client for the Tailscale REST API (`api.tailscale.com/api/v2`) using `Authorization: Bearer` header
- `TailscaleService` — business service layer with destructive gate (`allow_destructive` + `confirm=true` two-key interlock)
- Single MCP tool `tailscale` with action dispatch:
  - Read: `devices`, `device`, `device_routes`, `keys`, `acl`, `dns`, `users`
  - Write: `authorize_device`
  - Destructive: `delete_device`
  - Meta: `help`
- `dns` action aggregates nameservers, search paths, and MagicDNS preferences in one call using `tokio::try_join!`
- Streamable HTTP transport via Axum on port 7575
- stdio transport via RMCP for Claude Desktop / claude_desktop_config.json
- Auth modes: bearer token (default) and OAuth (Google PKCE flow with JWT issuance)
- Loopback / no-auth mode for local development
- CLI subcommands mirroring all MCP actions: `devices`, `device`, `routes`, `keys`, `acl`, `dns`, `users`, `authorize`, `delete-device --confirm`
- Config loading from `config.toml` with `TAILSCALE_*` env var overrides
- Test suite: CLI parsing, destructive gate, MCP tool dispatch (all in-process, no live Tailscale account required)
