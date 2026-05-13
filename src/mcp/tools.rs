use serde_json::{json, Value};

use super::AppState;

/// Thin shim — parse args, call service, return Value. No logic here.
pub(super) async fn execute_tool(
    state: &AppState,
    name: &str,
    args: Value,
) -> anyhow::Result<Value> {
    match name {
        "tailscale" => dispatch(state, args).await,
        _ => Err(anyhow::anyhow!("unknown tool: {name}")),
    }
}

async fn dispatch(state: &AppState, args: Value) -> anyhow::Result<Value> {
    let action =
        string_arg(&args, "action").ok_or_else(|| anyhow::anyhow!("action is required"))?;

    match action.as_str() {
        // Read
        "devices" => state.service.devices().await,
        "device" => {
            let id = require_id(&args, "device")?;
            state.service.device(&id).await
        }
        "device_routes" => {
            let id = require_id(&args, "device_routes")?;
            state.service.device_routes(&id).await
        }
        "keys" => state.service.keys().await,
        "acl" => state.service.acl().await,
        "dns" => state.service.dns().await,
        "users" => state.service.users().await,

        // Write
        "authorize_device" => {
            let id = require_id(&args, "authorize_device")?;
            state.service.authorize_device(&id).await
        }

        // Destructive
        "delete_device" => {
            let id = require_id(&args, "delete_device")?;
            let confirm = bool_arg(&args, "confirm").unwrap_or(false);
            state.service.delete_device(&id, confirm).await
        }

        // Meta
        "help" => Ok(json!({ "help": HELP_TEXT })),

        other => Err(anyhow::anyhow!(
            "unknown tailscale action: {other}; use action=help for documentation"
        )),
    }
}

fn string_arg(args: &Value, name: &str) -> Option<String> {
    args.get(name).and_then(|v| v.as_str()).map(String::from)
}

fn bool_arg(args: &Value, name: &str) -> Option<bool> {
    args.get(name).and_then(|v| v.as_bool())
}

fn require_id(args: &Value, action: &str) -> anyhow::Result<String> {
    string_arg(args, "id").ok_or_else(|| anyhow::anyhow!("`id` is required for {action}"))
}

const HELP_TEXT: &str = r#"# tailscale MCP Tool

Access your Tailscale network via the Tailscale REST API.
Set the required `action` argument to select the operation.

## Read
- `devices`          — List all devices in the tailnet
- `device`           — Single device details (requires `id`)
- `device_routes`    — Subnet routes for a device (requires `id`)
- `keys`             — List API keys in the tailnet
- `acl`              — ACL policy for the tailnet
- `dns`              — DNS nameservers, search paths, and preferences
- `users`            — Users in the tailnet

## Write
- `authorize_device` — Approve a device for the tailnet (requires `id`)

## Destructive (requires TAILSCALE_ALLOW_DESTRUCTIVE=true and confirm=true)
- `delete_device`    — Remove a device permanently (requires `id`, `confirm=true`)

## Meta
- `help`             — This documentation

## Notes
- The `tailnet` parameter is set server-side via TAILSCALE_TAILNET.
  Use "-" for personal accounts or your org domain (e.g. "example.com").
- Device IDs can be the stable node ID or the legacy numeric ID.
"#;
