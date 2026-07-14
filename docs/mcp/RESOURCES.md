# MCP Resources

The server advertises one resource:

```text
tailscale://schema/mcp-tool
```

It returns the current JSON tool definition list from `src/mcp/schemas.rs`.

Resource metadata includes:

- title: `Tailscale Tool Schema`
- MIME type: `application/json`
- size when serializable
- decorative icon
- `_meta.ai.dinglebear/tailscale-rmcp`

Use `resources/list` to discover it and `resources/read` to fetch it.
