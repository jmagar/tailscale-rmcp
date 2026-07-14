# Release And Metadata Checklist

Run before cutting or validating a release:

```bash
cargo fmt --check
cargo test --locked
cargo clippy -- -D warnings
npm --prefix packages/tailscale-rmcp run check
npm pack --prefix packages/tailscale-rmcp --dry-run --json
mcp-publisher validate server.json
git diff --check
```

Version surfaces that must agree:

- `Cargo.toml`
- `Cargo.lock`
- `.release-please-manifest.json`
- `packages/tailscale-rmcp/package.json`
- `server.json`
- `README.md`
- `packages/tailscale-rmcp/README.md`

Metadata surfaces to verify:

- `server.json.name = ai.dinglebear/tailscale-rmcp`
- npm `mcpName = ai.dinglebear/tailscale-rmcp`
- `serverInfo.name = tailscale-rmcp`
- `serverInfo.title = Tailscale RMCP`
- remote endpoint `https://ts.tootie.tv/mcp`
- tool/resource/prompt icons and `_meta`
- tool output schema `{ ok, data, error }`

Live smoke after deployment:

```bash
mcporter tools --config config/mcporter.json tailscale-rmcp
mcporter call --config config/mcporter.json tailscale-rmcp.tailscale action=devices
```
