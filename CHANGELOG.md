# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.2](https://github.com/jmagar/tailscale-rmcp/compare/v0.1.1...v0.1.2) (2026-07-09)


### Fixed

* **ci:** switch OpenWiki to local openai-compatible proxy ([e44a480](https://github.com/jmagar/tailscale-rmcp/commit/e44a48086965d7ef825d74ada9f69139dd1a98bb))

## [0.1.1] - 2026-06-01

### Changed

- Plugin `SessionStart`/`ConfigChange` hooks now call `${CLAUDE_PLUGIN_ROOT}/bin/rtailscale setup plugin-hook` directly instead of going through the `plugin-setup.sh` shell wrapper. The env-var mapping the script performed (`CLAUDE_PLUGIN_OPTION_*` → `TAILSCALE_*`, plus `CLAUDE_PLUGIN_DATA` → `TAILSCALE_MCP_HOME`) now lives in `apply_plugin_options()` in `src/setup.rs`, applied at the top of the plugin-hook path. The script's `.env`-fallback was dropped (immaterial: the binary never persists option values to `.env` and the setup checks read live process env).

### Removed

- `plugins/tailscale/hooks/plugin-setup.sh` — the wrapper was a pure env-mapping middleman now handled by the binary's `setup plugin-hook` command.

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
