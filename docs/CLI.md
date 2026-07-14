# Direct CLI Reference

The shipped binary is `rtailscale`. The npm package also exposes
`tailscale-rmcp` as an alias. The binary avoids the name `tailscale` so it does
not shadow the official Tailscale CLI.

All commands currently print JSON. `--json` and `-j` are accepted for parity
with the RMCP family.

## Read commands

```bash
rtailscale devices [--json]
rtailscale device <id> [--json]
rtailscale routes <device-id> [--json]
rtailscale keys [--json]
rtailscale acl [--json]
rtailscale dns [--json]
rtailscale users [--json]
```

## Write commands

```bash
rtailscale authorize <device-id> [--json]
```

## Destructive commands

```bash
TAILSCALE_ALLOW_DESTRUCTIVE=true rtailscale delete-device <device-id> --confirm [--json]
```

Both the server setting and `--confirm` are required.

## Runtime commands

```bash
rtailscale serve
rtailscale serve mcp
rtailscale mcp
rtailscale doctor [--json]
rtailscale setup check [--json]
rtailscale setup repair [--json]
rtailscale setup plugin-hook [--no-repair] [--json]
```

`serve` starts Streamable HTTP MCP on `TAILSCALE_MCP_HOST:TAILSCALE_MCP_PORT`.
`mcp` starts stdio MCP for local clients.

## MCP parity

| CLI command | MCP call |
|---|---|
| `rtailscale devices` | `tailscale(action="devices")` |
| `rtailscale device <id>` | `tailscale(action="device", id="<id>")` |
| `rtailscale routes <id>` | `tailscale(action="device_routes", id="<id>")` |
| `rtailscale keys` | `tailscale(action="keys")` |
| `rtailscale acl` | `tailscale(action="acl")` |
| `rtailscale dns` | `tailscale(action="dns")` |
| `rtailscale users` | `tailscale(action="users")` |
| `rtailscale authorize <id>` | `tailscale(action="authorize_device", id="<id>")` |
| `rtailscale delete-device <id> --confirm` | `tailscale(action="delete_device", id="<id>", confirm=true)` |

Both shims delegate to `TailscaleService`; policy does not live in the CLI.
