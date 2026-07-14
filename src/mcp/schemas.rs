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
        "title": "Tailscale",
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
        },
        "outputSchema": {
            "type": "object",
            "properties": {
                "ok": {
                    "type": "boolean",
                    "description": "True when the action completed successfully."
                },
                "data": {
                    "description": "Action result payload. Null when ok is false."
                },
                "error": {
                    "description": "Structured error details. Null when ok is true.",
                    "anyOf": [
                        { "type": "null" },
                        {
                            "type": "object",
                            "properties": {
                                "code": {
                                    "type": "string",
                                    "description": "Stable machine-readable error code."
                                },
                                "message": {
                                    "type": "string",
                                    "description": "Human-readable error message."
                                },
                                "status": {
                                    "type": "integer",
                                    "description": "HTTP-style status code."
                                },
                                "action": {
                                    "type": "string",
                                    "description": "Requested Tailscale action."
                                },
                                "upstream": {
                                    "type": "string",
                                    "description": "Upstream service that produced the error."
                                },
                                "hint": {
                                    "type": "string",
                                    "description": "Optional recovery hint."
                                },
                                "body": {
                                    "type": "string",
                                    "description": "Optional upstream response body for diagnostics."
                                }
                            },
                            "required": ["code", "message", "status", "action", "upstream"]
                        }
                    ]
                }
            },
            "required": ["ok", "data", "error"]
        },
        "annotations": {
            "title": "Tailscale",
            "readOnlyHint": false,
            "destructiveHint": true,
            "idempotentHint": false,
            "openWorldHint": true
        }
    })]
}
