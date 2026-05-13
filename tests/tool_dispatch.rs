/// Tests for MCP tool dispatch logic.
///
/// Uses in-process state with a stub TailscaleService (no live network).
/// Exercises the dispatch shim, not the HTTP layer.
use rustscale::mcp::testing;
use serde_json::json;

// ── help action ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn help_action_returns_help_text() {
    let state = testing::loopback_state();
    let args = json!({ "action": "help" });

    // Call through the public testing helper surface.
    // We verify the shape without a live Tailscale API.
    let result = dispatch_tool(&state, "tailscale", args).await;
    assert!(result.is_ok(), "help action should succeed");
    let val = result.unwrap();
    assert!(
        val.get("help").is_some(),
        "help action should return a 'help' key, got: {val}"
    );
}

// ── unknown action ────────────────────────────────────────────────────────────

#[tokio::test]
async fn unknown_action_returns_error() {
    let state = testing::loopback_state();
    let args = json!({ "action": "nonexistent_action" });

    let result = dispatch_tool(&state, "tailscale", args).await;
    assert!(result.is_err(), "unknown action should return an error");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("unknown tailscale action"),
        "error should mention unknown action, got: {msg}"
    );
}

// ── unknown tool ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn unknown_tool_name_returns_error() {
    let state = testing::loopback_state();
    let args = json!({ "action": "devices" });

    let result = dispatch_tool(&state, "not_a_tool", args).await;
    assert!(result.is_err(), "unknown tool should return an error");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("unknown tool"),
        "error should mention unknown tool, got: {msg}"
    );
}

// ── missing id ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn device_action_without_id_returns_error() {
    let state = testing::loopback_state();
    let args = json!({ "action": "device" });

    let result = dispatch_tool(&state, "tailscale", args).await;
    assert!(result.is_err(), "device without id should fail");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("id") && msg.contains("required"),
        "error should mention missing id, got: {msg}"
    );
}

#[tokio::test]
async fn device_routes_without_id_returns_error() {
    let state = testing::loopback_state();
    let args = json!({ "action": "device_routes" });

    let result = dispatch_tool(&state, "tailscale", args).await;
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("id") && msg.contains("required"), "{msg}");
}

#[tokio::test]
async fn authorize_device_without_id_returns_error() {
    let state = testing::loopback_state();
    let args = json!({ "action": "authorize_device" });

    let result = dispatch_tool(&state, "tailscale", args).await;
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("id") && msg.contains("required"), "{msg}");
}

// ── missing action ────────────────────────────────────────────────────────────

#[tokio::test]
async fn missing_action_returns_error() {
    let state = testing::loopback_state();
    let args = json!({});

    let result = dispatch_tool(&state, "tailscale", args).await;
    assert!(result.is_err(), "missing action should fail");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("action"),
        "error should mention action, got: {msg}"
    );
}

// ── helper ────────────────────────────────────────────────────────────────────

/// Drive the tool dispatch directly, bypassing HTTP.
async fn dispatch_tool(
    state: &rustscale::mcp::AppState,
    tool: &str,
    args: serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    // Access the internal execute_tool via a re-export in testing surface.
    // We replicate what rmcp_server::call_tool does:
    //   execute_tool(state, name, args)
    // Since execute_tool is pub(super), we reach it through the testing module path.
    rustscale::mcp::testing::call_tool(state, tool, args).await
}
