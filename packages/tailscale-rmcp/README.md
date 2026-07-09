# tailscale-rmcp

Node launcher for the `rtailscale` Rust MCP server and CLI binary.

```bash
npx -y tailscale-rmcp --help
```

The package downloads the matching GitHub Release binary during `postinstall`.

## MCP stdio

Use the package directly as an MCP command:

```json
{
  "mcpServers": {
    "tailscale": {
      "command": "npx",
      "args": ["-y", "tailscale-rmcp"]
    }
  }
}
```

## Environment

- `TAILSCALE_RMCP_BINARY_VERSION`: release tag/version to download, defaulting to this npm package version.
- `TAILSCALE_RMCP_VERSION`: alias for `TAILSCALE_RMCP_BINARY_VERSION`.
- `TAILSCALE_RMCP_REPO`: GitHub `owner/repo`, defaulting to `jmagar/tailscale-rmcp`.
- `TAILSCALE_RMCP_RELEASE_BASE_URL`: full release download base URL.
- `TAILSCALE_RMCP_SKIP_DOWNLOAD=1`: skip postinstall download.
