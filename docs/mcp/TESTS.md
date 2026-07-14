# MCP Tests

Local checks:

```bash
cargo fmt --check
cargo test --locked
cargo clippy -- -D warnings
npm --prefix packages/tailscale-rmcp run check
mcp-publisher validate server.json
git diff --check
```

Targeted tests:

```bash
cargo test --locked tool_dispatch
cargo test --locked destructive_gate
cargo test --locked advertised_surfaces_include_icons_meta_and_execution_metadata
```

Live HTTP checks:

```bash
curl -sf https://ts.tootie.tv/health
mcporter tools --config config/mcporter.json tailscale-rmcp
mcporter call --config config/mcporter.json tailscale-rmcp.tailscale action=devices
```

Live checks require a configured API key and endpoint auth.
