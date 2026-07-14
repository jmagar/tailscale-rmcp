# mcporter Smoke Tests

The repo mcporter config points at:

```text
https://ts.tootie.tv/mcp
```

List tools:

```bash
mcporter tools --config config/mcporter.json tailscale-rmcp
```

Read-only smoke:

```bash
mcporter call --config config/mcporter.json tailscale-rmcp.tailscale action=devices
mcporter call --config config/mcporter.json tailscale-rmcp.tailscale action=keys
mcporter call --config config/mcporter.json tailscale-rmcp.tailscale action=acl
mcporter call --config config/mcporter.json tailscale-rmcp.tailscale action=dns
mcporter call --config config/mcporter.json tailscale-rmcp.tailscale action=users
mcporter call --config config/mcporter.json tailscale-rmcp.tailscale action=help
```

Parameterized read-only actions need a real device ID:

```bash
mcporter call --config config/mcporter.json tailscale-rmcp.tailscale action=device id="$TAILSCALE_TEST_DEVICE_ID"
mcporter call --config config/mcporter.json tailscale-rmcp.tailscale action=device_routes id="$TAILSCALE_TEST_DEVICE_ID"
```

Do not include `authorize_device` or `delete_device` in default read-only smoke.
