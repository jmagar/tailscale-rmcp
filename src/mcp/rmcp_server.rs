use std::{borrow::Cow, net::Ipv6Addr, sync::Arc, time::Instant};

use lab_auth::AuthContext;
use rmcp::{
    model::{
        CallToolRequestParams, CallToolResult, Content, GetPromptRequestParams, GetPromptResult,
        Implementation, ListPromptsResult, ListResourcesResult, ListToolsResult,
        PaginatedRequestParams, RawResource, ReadResourceRequestParams, ReadResourceResult,
        Resource, ResourceContents, ServerCapabilities, ServerInfo, Tool,
    },
    service::RequestContext,
    transport::streamable_http_server::{
        session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
    },
    ErrorData, RoleServer, ServerHandler,
};
use serde_json::{Map, Value};

use crate::{config::McpConfig, tailscale::TailscaleApiError};

use super::{prompts, schemas::tool_definitions, tools::execute_tool, AppState, AuthPolicy};

const READ_SCOPE: &str = "tailscale:read";
const WRITE_SCOPE: &str = "tailscale:write";
const DENY_SCOPE: &str = "tailscale:__deny__";

const READ_ONLY_ACTIONS: &[&str] = &[
    "devices",
    "device",
    "device_routes",
    "keys",
    "acl",
    "dns",
    "users",
];

const WRITE_ACTIONS: &[&str] = &["authorize_device"];

#[derive(Clone)]
pub struct TailscaleRmcpServer {
    state: AppState,
}

pub fn rmcp_server(state: AppState) -> TailscaleRmcpServer {
    TailscaleRmcpServer { state }
}

impl ServerHandler for TailscaleRmcpServer {
    // ── tools ─────────────────────────────────────────────────────────────────

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        require_auth_context(&self.state, &context)?;
        let tools = rmcp_tool_definitions()?;
        tracing::info!(tool_count = tools.len(), "MCP tools listed");
        Ok(ListToolsResult {
            tools,
            ..Default::default()
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let tool_name = request.name.to_string();

        let action: String = request
            .arguments
            .as_ref()
            .and_then(|m| m.get("action"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();

        let auth = require_auth_context(&self.state, &context)?;
        if let (Some(auth), Some(required_scope)) = (auth, required_scope_for(&action)) {
            check_scope(auth, required_scope, &action)?;
        }

        let arguments = request
            .arguments
            .map(Value::Object)
            .unwrap_or_else(|| Value::Object(Map::new()));

        let started = Instant::now();
        tracing::info!(tool = %tool_name, action = %action, "MCP tool execution started");

        match execute_tool(&self.state, &tool_name, arguments).await {
            Ok(result) => {
                tracing::info!(
                    tool = %tool_name,
                    elapsed_ms = started.elapsed().as_millis(),
                    "MCP tool execution completed"
                );
                tool_result_from_json(result)
            }
            Err(error) if is_validation_error(&error) => {
                tracing::warn!(
                    tool = %tool_name,
                    elapsed_ms = started.elapsed().as_millis(),
                    "MCP tool rejected invalid params"
                );
                Err(ErrorData::invalid_params(error.to_string(), None))
            }
            Err(error) => {
                tracing::error!(
                    tool = %tool_name,
                    elapsed_ms = started.elapsed().as_millis(),
                    error = %error,
                    "MCP tool execution failed"
                );
                Ok(tool_error_result(&action, &error))
            }
        }
    }

    // ── resources ─────────────────────────────────────────────────────────────

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        require_auth_context(&self.state, &context)?;
        Ok(ListResourcesResult {
            resources: vec![schema_resource()],
            ..Default::default()
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        require_auth_context(&self.state, &context)?;
        if request.uri != SCHEMA_RESOURCE_URI {
            return Err(ErrorData::invalid_params(
                format!("unknown resource: {}", request.uri),
                None,
            ));
        }
        let schema = tool_definitions();
        let text = serde_json::to_string_pretty(&schema)
            .map_err(|e| ErrorData::internal_error(format!("serialization error: {e}"), None))?;
        Ok(ReadResourceResult::new(vec![ResourceContents::text(
            text,
            SCHEMA_RESOURCE_URI,
        )
        .with_mime_type("application/json")]))
    }

    // ── prompts ───────────────────────────────────────────────────────────────

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, ErrorData> {
        require_auth_context(&self.state, &context)?;
        Ok(prompts::list_prompts())
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, ErrorData> {
        require_auth_context(&self.state, &context)?;
        prompts::get_prompt(request).map_err(|e| ErrorData::invalid_params(e.to_string(), None))
    }

    // ── server info ───────────────────────────────────────────────────────────

    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_prompts()
                .build(),
        )
        .with_server_info(Implementation::new(
            self.state.config.server_name.clone(),
            env!("CARGO_PKG_VERSION"),
        ))
    }
}

// ── transport helpers ─────────────────────────────────────────────────────────

pub fn streamable_http_config(config: &McpConfig) -> StreamableHttpServerConfig {
    StreamableHttpServerConfig::default()
        .with_stateful_mode(false)
        .with_json_response(true)
        .with_allowed_hosts(allowed_hosts(config))
        .with_allowed_origins(allowed_origins(config))
}

pub fn streamable_http_service(
    state: AppState,
    config: StreamableHttpServerConfig,
) -> StreamableHttpService<TailscaleRmcpServer, LocalSessionManager> {
    StreamableHttpService::new(
        move || {
            Ok(TailscaleRmcpServer {
                state: state.clone(),
            })
        },
        Default::default(),
        config,
    )
}

// ── resource definitions ──────────────────────────────────────────────────────

const SCHEMA_RESOURCE_URI: &str = "tailscale://schema/mcp-tool";

fn schema_resource() -> Resource {
    Resource::new(
        RawResource::new(SCHEMA_RESOURCE_URI, "tailscale tool schema")
            .with_description(
                "JSON schema for the tailscale MCP tool and its action-based parameters",
            )
            .with_mime_type("application/json"),
        None,
    )
}

// ── tool definition conversion ────────────────────────────────────────────────

fn rmcp_tool_definitions() -> Result<Vec<Tool>, ErrorData> {
    tool_definitions()
        .into_iter()
        .map(rmcp_tool_from_json)
        .collect()
}

fn rmcp_tool_from_json(value: Value) -> Result<Tool, ErrorData> {
    let name = value
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| ErrorData::internal_error("tool definition missing name", None))?;
    let description = value
        .get("description")
        .and_then(Value::as_str)
        .map(|d| Cow::Owned(d.to_string()));
    let input_schema = value
        .get("inputSchema")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| ErrorData::internal_error("tool definition missing inputSchema", None))?;
    Ok(Tool::new_with_raw(
        Cow::Owned(name.to_string()),
        description,
        Arc::new(input_schema),
    ))
}

fn tool_result_from_json(value: Value) -> Result<CallToolResult, ErrorData> {
    let envelope = success_envelope(value);
    let text = serde_json::to_string_pretty(&envelope)
        .map_err(|e| ErrorData::internal_error(format!("serialization error: {e}"), None))?;
    let mut result = CallToolResult::success(vec![Content::text(text)]);
    result.structured_content = Some(envelope);
    Ok(result)
}

fn tool_error_result(action: &str, error: &anyhow::Error) -> CallToolResult {
    if let Some(api_error) = error.downcast_ref::<TailscaleApiError>() {
        return api_error_tool_result(action, api_error);
    }

    let message =
        format!("Tool execution failed for action '{action}'. Check server logs for details.");
    structured_error_result(action, "tool_execution_failed", 500, &message, None, None)
}

fn api_error_tool_result(action: &str, error: &TailscaleApiError) -> CallToolResult {
    structured_error_result(
        action,
        error.code,
        error.status.as_u16(),
        &error.message,
        error.hint.as_deref(),
        non_empty(error.body.as_str()),
    )
}

fn structured_error_result(
    action: &str,
    code: &str,
    status: u16,
    message: &str,
    hint: Option<&str>,
    body: Option<&str>,
) -> CallToolResult {
    let mut error = Map::new();
    error.insert("code".to_string(), Value::String(code.to_string()));
    error.insert("message".to_string(), Value::String(message.to_string()));
    error.insert("status".to_string(), Value::from(status));
    error.insert("action".to_string(), Value::String(action.to_string()));
    error.insert(
        "upstream".to_string(),
        Value::String("tailscale".to_string()),
    );
    if let Some(hint) = hint {
        error.insert("hint".to_string(), Value::String(hint.to_string()));
    }
    if let Some(body) = body {
        error.insert("body".to_string(), Value::String(body.to_string()));
    }

    let payload = error_envelope(Value::Object(error));
    let text = serde_json::to_string_pretty(&payload).unwrap_or_else(|_| message.to_string());

    let mut result = CallToolResult::structured_error(payload);
    result.content = vec![Content::text(text)];
    result
}

fn success_envelope(data: Value) -> Value {
    let mut payload = Map::new();
    payload.insert("ok".to_string(), Value::Bool(true));
    payload.insert("data".to_string(), data);
    payload.insert("error".to_string(), Value::Null);
    Value::Object(payload)
}

fn error_envelope(error: Value) -> Value {
    let mut payload = Map::new();
    payload.insert("ok".to_string(), Value::Bool(false));
    payload.insert("data".to_string(), Value::Null);
    payload.insert("error".to_string(), error);
    Value::Object(payload)
}

fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

// ── auth helpers ──────────────────────────────────────────────────────────────

fn require_auth_context<'a>(
    state: &AppState,
    ctx: &'a RequestContext<RoleServer>,
) -> Result<Option<&'a AuthContext>, ErrorData> {
    match &state.auth_policy {
        AuthPolicy::LoopbackDev => Ok(None),
        AuthPolicy::Mounted { .. } => {
            let parts = ctx
                .extensions
                .get::<axum::http::request::Parts>()
                .ok_or_else(|| {
                    tracing::error!(
                        "rmcp HTTP Parts extension absent — middleware ordering may be broken"
                    );
                    ErrorData::invalid_request("forbidden: missing http context", None)
                })?;
            let auth = parts.extensions.get::<AuthContext>().ok_or_else(|| {
                tracing::warn!("AuthContext absent — AuthLayer may not be mounted");
                ErrorData::invalid_request("forbidden: missing auth context", None)
            })?;
            Ok(Some(auth))
        }
    }
}

fn check_scope(auth: &AuthContext, required_scope: &str, action: &str) -> Result<(), ErrorData> {
    let satisfied = auth.scopes.iter().any(|s| {
        s == required_scope
            || s == "tailscale:admin"
            || (required_scope == READ_SCOPE && s == WRITE_SCOPE)
    });
    if satisfied {
        return Ok(());
    }
    tracing::warn!(
        subject = %auth.sub,
        action = %action,
        required_scope = %required_scope,
        "MCP tool denied: insufficient scope"
    );
    Err(ErrorData::invalid_request(
        format!("forbidden: requires scope: {required_scope}"),
        None,
    ))
}

fn required_scope_for(action: &str) -> Option<&'static str> {
    if action == "help" {
        None
    } else if READ_ONLY_ACTIONS.contains(&action) {
        Some(READ_SCOPE)
    } else if WRITE_ACTIONS.contains(&action) {
        Some(WRITE_SCOPE)
    } else {
        // delete_device and unknowns → deny
        Some(DENY_SCOPE)
    }
}

fn is_validation_error(error: &anyhow::Error) -> bool {
    let msg = error.to_string();
    msg.contains("is required") || msg.contains("unknown tailscale action")
}

// ── allowed hosts / origins ──────────────────────────────────────────────────

pub(super) fn allowed_hosts(config: &McpConfig) -> Vec<String> {
    let mut hosts = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    push_host_variants(&mut hosts, &config.host, config.port);
    push_host_variants(&mut hosts, "localhost", config.port);
    push_host_variants(&mut hosts, "127.0.0.1", config.port);
    push_host_variants(&mut hosts, "::1", config.port);
    for host in &config.allowed_hosts {
        push_host_variants(&mut hosts, host, config.port);
    }
    if let Some(public_url) = config.auth.public_url.as_deref() {
        push_public_url_hosts(&mut hosts, public_url, config.port);
    }
    hosts.sort();
    hosts.dedup();
    hosts
}

pub(super) fn allowed_origins(config: &McpConfig) -> Vec<String> {
    let mut origins = vec![
        format!("http://localhost:{}", config.port),
        format!("http://127.0.0.1:{}", config.port),
    ];
    origins.extend(config.allowed_origins.iter().cloned());
    if let Some(public_url) = config.auth.public_url.as_deref() {
        if let Some(origin) = extract_origin(public_url) {
            origins.push(origin);
        }
    }
    origins.sort();
    origins.dedup();
    origins
}

fn push_host_variants(hosts: &mut Vec<String>, host: &str, port: u16) {
    let host = host.trim();
    if host.is_empty() {
        return;
    }
    hosts.push(host.to_string());
    if host.starts_with('[') && host.contains("]:") {
        return;
    }
    if let Some(inner) = host.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
        if !inner.is_empty() {
            hosts.push(format!("[{inner}]:{port}"));
        }
    } else if host.parse::<Ipv6Addr>().is_ok() {
        hosts.push(format!("[{host}]"));
        hosts.push(format!("[{host}]:{port}"));
    } else if !has_port(host) {
        hosts.push(format!("{host}:{port}"));
    }
}

fn push_public_url_hosts(hosts: &mut Vec<String>, url: &str, listen_port: u16) {
    let Ok(parsed) = url::Url::parse(url) else {
        tracing::warn!(
            public_url = url,
            "TAILSCALE_MCP_PUBLIC_URL is not a valid URL"
        );
        return;
    };
    let Some(host) = parsed.host_str() else {
        return;
    };
    if host.contains('*') {
        tracing::warn!(
            host,
            "TAILSCALE_MCP_PUBLIC_URL host contains wildcard; skipping"
        );
        return;
    }
    let explicit_port = parsed.port();
    let scheme_default = match parsed.scheme() {
        "https" => Some(443u16),
        "http" => Some(80u16),
        _ => None,
    };
    if let Some(p) = explicit_port {
        push_host_variants(hosts, host, p);
        let with_port = format!("{host}:{p}");
        if !hosts.contains(&with_port) {
            hosts.push(with_port);
        }
    } else if let Some(default_port) = scheme_default {
        let bare = host.to_string();
        if !hosts.contains(&bare) {
            hosts.push(bare);
        }
        let with_default = format!("{host}:{default_port}");
        if !hosts.contains(&with_default) {
            hosts.push(with_default);
        }
    } else {
        push_host_variants(hosts, host, listen_port);
    }
}

fn has_port(host: &str) -> bool {
    host.rsplit_once(':')
        .and_then(|(_, p)| p.parse::<u16>().ok())
        .is_some()
}

fn extract_origin(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url)
        .map_err(
            |e| tracing::warn!(public_url = url, error = %e, "invalid TAILSCALE_MCP_PUBLIC_URL"),
        )
        .ok()?;
    let scheme = parsed.scheme();
    let host = parsed.host_str()?;
    if host.contains('*') {
        return None;
    }
    let default_port = match scheme {
        "http" => Some(80u16),
        "https" => Some(443u16),
        _ => None,
    };
    let origin = match parsed.port() {
        Some(port) if default_port != Some(port) => format!("{scheme}://{host}:{port}"),
        _ => format!("{scheme}://{host}"),
    };
    Some(origin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_tool_results_return_envelope() {
        let data = serde_json::json!({
            "devices": [
                { "hostname": "node-a" }
            ]
        });
        let result = tool_result_from_json(data.clone()).expect("tool result should serialize");

        assert_eq!(result.is_error, Some(false));
        let structured = result
            .structured_content
            .as_ref()
            .expect("success results should include structured envelope");
        assert_eq!(structured["ok"], true);
        assert_eq!(structured["data"], data);
        assert_eq!(structured["error"], Value::Null);

        let text = result
            .content
            .first()
            .and_then(|content| content.as_text())
            .map(|text| text.text.as_str())
            .expect("success result should include text content");
        let parsed_text: Value =
            serde_json::from_str(text).expect("success text should be the JSON envelope");
        assert_eq!(&parsed_text, structured);
    }

    #[test]
    fn api_not_found_errors_return_structured_mcp_error() {
        let error = anyhow::Error::new(TailscaleApiError {
            status: reqwest::StatusCode::NOT_FOUND,
            code: "not_found",
            message: "Device not found".to_string(),
            hint: Some("check the device ID".to_string()),
            body: r#"{"message":"missing"}"#.to_string(),
        });
        let result = tool_error_result("device", &error);

        assert_eq!(result.is_error, Some(true));
        let text = result
            .content
            .first()
            .and_then(|content| content.as_text())
            .map(|text| text.text.as_str());
        let parsed_text: Value = serde_json::from_str(text.expect("error should include text"))
            .expect("error text should be the JSON envelope");

        let structured = result
            .structured_content
            .as_ref()
            .expect("API errors should include structured content");
        assert_eq!(&parsed_text, structured);
        assert_eq!(structured["ok"], false);
        assert_eq!(structured["data"], Value::Null);
        assert_eq!(structured["error"]["message"], "Device not found");
        assert_eq!(structured["error"]["code"], "not_found");
        assert_eq!(structured["error"]["status"], 404);
        assert_eq!(structured["error"]["action"], "device");
        assert_eq!(structured["error"]["hint"], "check the device ID");
        assert_eq!(structured["error"]["body"], r#"{"message":"missing"}"#);
    }

    #[test]
    fn generic_tool_errors_return_envelope() {
        let error = anyhow::anyhow!("internal details stay in logs");
        let result = tool_error_result("devices", &error);

        assert_eq!(result.is_error, Some(true));
        let structured = result
            .structured_content
            .as_ref()
            .expect("generic errors should include structured envelope");
        assert_eq!(structured["ok"], false);
        assert_eq!(structured["data"], Value::Null);
        assert_eq!(structured["error"]["code"], "tool_execution_failed");
        assert_eq!(structured["error"]["action"], "devices");
    }
}
