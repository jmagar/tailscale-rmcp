use serde_json::json;
/// Tests for the destructive gate in TailscaleService.
///
/// These run in-process — no live Tailscale API needed.
/// The client is constructed with a stub key; calls are blocked by the gate
/// before they ever reach the network.
use tailscale_rmcp::mcp::testing;

// ── disabled server + no confirm ─────────────────────────────────────────────

#[tokio::test]
async fn delete_fails_when_server_disallows_destructive() {
    // loopback_state has allow_destructive = false
    let state = testing::loopback_state();
    let args = json!({ "action": "delete_device", "id": "dev-123", "confirm": true });

    let result = testing::call_tool(&state, "tailscale", args).await;
    assert!(result.is_err(), "should be blocked by server config");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("destructive operations are disabled"),
        "error should explain why: {msg}"
    );
}

// ── enabled server + no confirm ──────────────────────────────────────────────

#[tokio::test]
async fn delete_fails_without_confirm_even_when_destructive_enabled() {
    // destructive_state has allow_destructive = true
    let state = testing::destructive_state();
    let args = json!({ "action": "delete_device", "id": "dev-123", "confirm": false });

    let result = testing::call_tool(&state, "tailscale", args).await;
    assert!(result.is_err(), "should require confirm=true");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("confirm=true is required"),
        "error should request confirmation: {msg}"
    );
}

// ── enabled server + confirm omitted (defaults to false) ─────────────────────

#[tokio::test]
async fn delete_fails_when_confirm_omitted() {
    let state = testing::destructive_state();
    // confirm key absent — should default to false
    let args = json!({ "action": "delete_device", "id": "dev-123" });

    let result = testing::call_tool(&state, "tailscale", args).await;
    assert!(
        result.is_err(),
        "confirm absent should default to false → fail"
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("confirm=true is required"),
        "error should request confirmation: {msg}"
    );
}

// ── enabled server + confirm=true attempts network (expected to fail w/ API error)

#[tokio::test]
async fn delete_with_confirm_passes_gate_and_reaches_api() {
    let state = testing::destructive_state();
    let args = json!({ "action": "delete_device", "id": "dev-123", "confirm": true });

    let result = testing::call_tool(&state, "tailscale", args).await;
    // The gate passes, but the API call will fail because the key is "test"
    // and the device doesn't exist. Either a network error or API 401/404.
    // What matters: the error is NOT the gate error.
    if let Err(e) = &result {
        let msg = e.to_string();
        assert!(
            !msg.contains("destructive operations are disabled"),
            "should not be blocked by server config gate: {msg}"
        );
        assert!(
            !msg.contains("confirm=true is required"),
            "should not be blocked by confirm gate: {msg}"
        );
    }
    // If somehow it succeeded (unlikely with a fake key), that's fine too.
}

// ── non-destructive actions are unaffected ────────────────────────────────────

#[tokio::test]
async fn authorize_device_is_not_gated_by_allow_destructive() {
    // loopback_state has allow_destructive = false; authorize should not be blocked by that gate
    let state = testing::loopback_state();
    let args = json!({ "action": "authorize_device", "id": "dev-123" });

    let result = testing::call_tool(&state, "tailscale", args).await;
    // Will fail with a network/API error (fake key), NOT a destructive-gate error.
    if let Err(e) = &result {
        let msg = e.to_string();
        assert!(
            !msg.contains("destructive operations are disabled"),
            "authorize_device should not be blocked by destructive gate: {msg}"
        );
    }
}
