---
name: tailscale
description: >
  rustscale MCP server — query and manage your Tailscale network from Claude.
  Use this skill whenever the user mentions Tailscale devices, tailnet status,
  VPN nodes, MagicDNS, Tailscale ACL, access control policy, subnet routes,
  tailnet members, API keys, or wants to authorize or delete a device. Trigger
  phrases include: "list my Tailscale devices", "show tailnet status", "what's
  on my tailnet", "check my VPN devices", "Tailscale ACL", "MagicDNS",
  "Tailscale DNS", "tailnet users", "authorize this device", "delete device
  from Tailscale", "device routes", "subnet routes", "Tailscale API keys",
  "tailscale network". Always use this skill — do not try to query Tailscale
  without it.
---

# Tailscale Skill

Bridge to the Tailscale REST API (`https://api.tailscale.com/api/v2`) via the
**rustscale** MCP server. Exposes a single `tailscale` MCP tool with action
dispatch. Use the three tiers below in order: MCP first, CLI second, raw REST
only as a last resort.

---

## Tier 1 — MCP Tool (always try this first)

**Tool:** `tailscale`  
**Required parameter:** `action` (string)

### Read actions

These require only `action`; no `id` needed.

| Action | What it returns |
|--------|-----------------|
| `devices` | All devices in the tailnet — `nodeId`, `hostname`, `addresses` (IPs), `os`, `user`, `lastSeen`, `authorized`, `online` |
| `keys` | API keys — `id`, `expires`, `capabilities`, `description` |
| `acl` | ACL policy as parsed JSON — `acls`, `groups`, `tagOwners`, `hosts`, etc. |
| `dns` | Aggregated DNS info — `nameservers`, `searchpaths`, `preferences` (MagicDNS status) |
| `users` | Tailnet members — `loginName`, `displayName`, `role`, `status` |
| `help` | Full built-in documentation |

### Read actions requiring `id`

Pass the device's stable node ID (starts with `n`) or legacy numeric ID.

| Action | `id` | What it returns |
|--------|------|-----------------|
| `device` | required | Single device — same fields as `devices` plus `clientVersion`, `updateAvailable`, `blocksIncomingConnections`, `enabledRoutes`, `advertisedRoutes` |
| `device_routes` | required | Subnet routes — `advertisedRoutes` and `enabledRoutes` arrays |

### Write action (non-destructive)

| Action | `id` | Effect |
|--------|------|--------|
| `authorize_device` | required | Approves the device so it can join the tailnet; returns `{"ok": true}` |

### Destructive action — two-key interlock

`delete_device` permanently removes a device. It requires **both**:
1. `confirm=True` in the call
2. `TAILSCALE_ALLOW_DESTRUCTIVE=true` on the server

If either is missing the server returns a clear error — no device is touched.

```python
tailscale(action="delete_device", id="nXXXXXXXXXXXXX", confirm=True)
# → {"ok": true, "device_id": "nXXX...", "action": "deleted"}
```

### Quick-reference examples

```python
# List all devices
tailscale(action="devices")

# Inspect a single device
tailscale(action="device", id="nXXXXXXXXXXXXX")

# Check subnet routes
tailscale(action="device_routes", id="nXXXXXXXXXXXXX")

# List API keys
tailscale(action="keys")

# Read the ACL policy
tailscale(action="acl")

# DNS nameservers + search paths + MagicDNS in one call
tailscale(action="dns")

# Tailnet members
tailscale(action="users")

# Approve a pending device
tailscale(action="authorize_device", id="nXXXXXXXXXXXXX")

# Remove a device (both guards must be satisfied)
tailscale(action="delete_device", id="nXXXXXXXXXXXXX", confirm=True)
```

---

## Tier 2 — CLI Binary (fall back when MCP is unavailable)

Binary: `/home/jmagar/workspace/rustscale/target/release/rtailscale`  
All commands accept `--json` / `-j` for machine-readable JSON output.

```bash
rtailscale devices
rtailscale device <id>
rtailscale routes <id>
rtailscale keys
rtailscale acl
rtailscale dns
rtailscale users
rtailscale authorize <id>
rtailscale delete-device <id> --confirm
```

---

## Tier 3 — Direct Tailscale REST API (last resort)

Base URL: `https://api.tailscale.com/api/v2`  
Auth: `Authorization: Bearer $TAILSCALE_API_KEY`  
Tailnet: `$TAILSCALE_TAILNET` (use `-` for personal accounts, or your org domain e.g. `example.com`)

```bash
# List all devices
curl "https://api.tailscale.com/api/v2/tailnet/$TAILSCALE_TAILNET/devices" \
  -H "Authorization: Bearer $TAILSCALE_API_KEY"

# Single device
curl "https://api.tailscale.com/api/v2/device/$DEVICE_ID" \
  -H "Authorization: Bearer $TAILSCALE_API_KEY"

# Device subnet routes
curl "https://api.tailscale.com/api/v2/device/$DEVICE_ID/routes" \
  -H "Authorization: Bearer $TAILSCALE_API_KEY"

# ACL policy — must request JSON to avoid HuJSON format
curl "https://api.tailscale.com/api/v2/tailnet/$TAILSCALE_TAILNET/acl" \
  -H "Authorization: Bearer $TAILSCALE_API_KEY" \
  -H "Accept: application/json"

# DNS nameservers
curl "https://api.tailscale.com/api/v2/tailnet/$TAILSCALE_TAILNET/dns/nameservers" \
  -H "Authorization: Bearer $TAILSCALE_API_KEY"

# DNS search paths
curl "https://api.tailscale.com/api/v2/tailnet/$TAILSCALE_TAILNET/dns/searchpaths" \
  -H "Authorization: Bearer $TAILSCALE_API_KEY"

# DNS preferences (MagicDNS on/off, etc.)
curl "https://api.tailscale.com/api/v2/tailnet/$TAILSCALE_TAILNET/dns/preferences" \
  -H "Authorization: Bearer $TAILSCALE_API_KEY"

# Tailnet users
curl "https://api.tailscale.com/api/v2/tailnet/$TAILSCALE_TAILNET/users" \
  -H "Authorization: Bearer $TAILSCALE_API_KEY"

# API keys
curl "https://api.tailscale.com/api/v2/tailnet/$TAILSCALE_TAILNET/keys" \
  -H "Authorization: Bearer $TAILSCALE_API_KEY"

# Authorize a device (write)
curl -X POST "https://api.tailscale.com/api/v2/device/$DEVICE_ID/authorized" \
  -H "Authorization: Bearer $TAILSCALE_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"authorized":true}'

# Delete a device (irreversible — confirm with the user first)
curl -X DELETE "https://api.tailscale.com/api/v2/device/$DEVICE_ID" \
  -H "Authorization: Bearer $TAILSCALE_API_KEY"
```

---

## Key facts to keep in mind

- **Device IDs:** stable node IDs start with `n` (e.g. `nXXXXXXXXXXXXX`). Legacy
  numeric IDs are also accepted everywhere `id` is required.
- **Tailnet `-`:** the hyphen is a valid tailnet value meaning "personal account".
  The server resolves it from `TAILSCALE_TAILNET`.
- **`dns` aggregates three REST endpoints** (nameservers, searchpaths,
  preferences) into one MCP call — no need to call them separately.
- **ACL uses `Accept: application/json`** because the raw API returns HuJSON
  (a JSON superset with comments). The MCP server sets this header automatically;
  you only need it when falling back to raw `curl`.
- **Destructive gate — both keys required:** the server checks
  `TAILSCALE_ALLOW_DESTRUCTIVE=true` AND `confirm=true` independently. An error
  from either means the device was not deleted.
- **All other actions are read-only** except `authorize_device` (write) and
  `delete_device` (destructive).
