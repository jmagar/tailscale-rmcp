# Recipes

## Build

```bash
cargo build --release
```

## Run loopback HTTP

```bash
TAILSCALE_MCP_HOST=127.0.0.1 ./target/release/rtailscale serve
```

## Run stdio

```bash
./target/release/rtailscale mcp
```

## Verify package

```bash
npm --prefix packages/tailscale-rmcp run check
npm pack --prefix packages/tailscale-rmcp --dry-run --json
```

## Validate registry manifest

```bash
mcp-publisher validate server.json
```

## Refresh references

```bash
scripts/refresh-docs.sh
```
