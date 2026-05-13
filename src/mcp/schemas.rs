use serde_json::{json, Value};

pub(super) const TAILSCALE_ACTIONS: &[&str] = &[
    "devices",
    "device",
    "device_routes",
    "keys",
    "acl",
    "dns",
    "users",
    "authorize_device",
    "delete_device",
    "help",
];

pub(super) fn tool_definitions() -> Vec<Value> {
    vec![json!({
        "name": "tailscale",
        "description": "Query and manage your Tailscale network via the Tailscale REST API. Use action=help for documentation.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation to perform.",
                    "enum": TAILSCALE_ACTIONS
                },
                "id": {
                    "type": "string",
                    "description": "Device ID — required for: device, device_routes, authorize_device, delete_device."
                },
                "confirm": {
                    "type": "boolean",
                    "description": "Must be true to execute destructive operations (delete_device)."
                }
            },
            "required": ["action"]
        }
    })]
}
