# MCP Transports

| Transport | Entry point | Use case |
|---|---|---|
| stdio | `rtailscale mcp` | Local MCP clients that launch a child process. |
| Streamable HTTP | `rtailscale serve` or `rtailscale serve mcp` | Shared service, gateway, Docker, reverse proxy. |

HTTP configuration:

```bash
TAILSCALE_MCP_HOST=0.0.0.0
TAILSCALE_MCP_PORT=40040
TAILSCALE_MCP_TOKEN=change-me
```

The RMCP server runs stateless JSON-response mode and exposes:

```text
POST /mcp
GET  /health
```

OAuth mode also exposes discovery routes under `/mcp/.well-known/...`.
