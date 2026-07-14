use rmcp::model::{
    GetPromptRequestParams, GetPromptResult, ListPromptsResult, Prompt, PromptMessage,
    PromptMessageRole,
};

pub(super) fn list_prompts() -> ListPromptsResult {
    ListPromptsResult {
        prompts: vec![Prompt::new(
            "network_status",
            Some("Check all Tailscale devices and summarize tailnet health."),
            None,
        )
        .with_title("Network Status")],
        ..Default::default()
    }
}

pub(super) fn get_prompt(request: GetPromptRequestParams) -> anyhow::Result<GetPromptResult> {
    match request.name.as_str() {
        "network_status" => Ok(GetPromptResult::new(vec![PromptMessage::new_text(
            PromptMessageRole::User,
            "Use the tailscale tool with action=devices to retrieve all devices in the tailnet. \
             Then summarize: total device count, how many are online vs offline, any devices \
             that appear to have lost connectivity or have expired keys, and any unusual route \
             configurations. Also note the tailnet's ACL and DNS configuration if relevant.",
        )])
        .with_description("Check all Tailscale devices and summarize tailnet health")),
        other => Err(anyhow::anyhow!("unknown prompt: {other}")),
    }
}
