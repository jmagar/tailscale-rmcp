# MCP Tool Reference

One tool is exposed:

```text
tailscale
```

The required `action` argument selects the operation.

## Read actions

| Action | Required params | Description |
|---|---|---|
| `devices` | none | List devices in the tailnet. |
| `device` | `id` | Return one device. |
| `device_routes` | `id` | Return routes for one device. |
| `keys` | none | List API keys. |
| `acl` | none | Return ACL policy JSON. |
| `dns` | none | Return DNS settings. |
| `users` | none | List users. |
| `help` | none | Return action help. |

## Write actions

| Action | Required params | Description |
|---|---|---|
| `authorize_device` | `id` | Authorize a device. |

## Destructive actions

| Action | Required params | Description |
|---|---|---|
| `delete_device` | `id`, `confirm=true` | Delete a device when server destructive mode is enabled. |

## Example

```json
{
  "name": "tailscale",
  "arguments": {
    "action": "device",
    "id": "n1234567890"
  }
}
```
