# Architecture

```text
MCP client / CLI
       |
       v
src/mcp/tools.rs or src/cli.rs
       |
       v
src/app.rs - TailscaleService
       |
       v
src/tailscale.rs - TailscaleClient
       |
       v
https://api.tailscale.com/api/v2
```

HTTP MCP is mounted with Axum under `/mcp`. The RMCP handler advertises tools,
resources, prompts, server info, icons, and `_meta`.

The service exposes one action-dispatched tool. Response shaping happens at the
MCP result boundary so every tool call returns the same `{ ok, data, error }`
shape.

Destructive policy lives in `TailscaleService::delete_device`.
