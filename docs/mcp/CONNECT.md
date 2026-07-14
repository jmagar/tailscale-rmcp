# Connect To MCP

## Stdio

```json
{
  "mcpServers": {
    "tailscale": {
      "command": "npx",
      "args": ["-y", "tailscale-rmcp", "mcp"],
      "env": {
        "TAILSCALE_API_KEY": "tskey-api-...",
        "TAILSCALE_TAILNET": "-"
      }
    }
  }
}
```

## HTTP loopback

Start:

```bash
TAILSCALE_MCP_HOST=127.0.0.1 rtailscale serve
```

Client:

```json
{
  "mcpServers": {
    "tailscale": {
      "type": "http",
      "url": "http://127.0.0.1:40040/mcp"
    }
  }
}
```

## HTTP bearer

```json
{
  "mcpServers": {
    "tailscale": {
      "type": "http",
      "url": "https://ts.tootie.tv/mcp",
      "headers": {
        "Authorization": "Bearer ${TAILSCALE_MCP_TOKEN}"
      }
    }
  }
}
```

## Raw JSON-RPC smoke

```bash
curl -s https://ts.tootie.tv/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -H "Authorization: Bearer $TAILSCALE_MCP_TOKEN" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}'
```
