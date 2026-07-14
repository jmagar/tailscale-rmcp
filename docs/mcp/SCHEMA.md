# Tool Schema

The `tailscale` tool has one required input field:

```json
{
  "type": "object",
  "required": ["action"],
  "properties": {
    "action": {
      "type": "string",
      "enum": ["devices", "device", "device_routes", "keys", "acl", "dns", "users", "authorize_device", "delete_device", "help"]
    },
    "id": {
      "type": "string"
    },
    "confirm": {
      "type": "boolean"
    }
  }
}
```

Every MCP tool result uses:

```json
{
  "ok": true,
  "data": {},
  "error": null
}
```

or:

```json
{
  "ok": false,
  "data": null,
  "error": {
    "code": "not_found",
    "message": "Device not found",
    "status": 404,
    "action": "device",
    "upstream": "tailscale"
  }
}
```

Advertised metadata includes:

- title and annotations
- output schema for `{ ok, data, error }`
- `execution.taskSupport = forbidden`
- decorative icons
- `_meta.ai.dinglebear/tailscale-rmcp`

The live schema is available as the MCP resource
`tailscale://schema/mcp-tool`.
