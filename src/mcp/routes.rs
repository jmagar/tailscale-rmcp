use std::sync::Arc;

use axum::{
    http::{HeaderValue, Method, StatusCode},
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use serde_json::json;
use tower_http::{
    cors::{Any, CorsLayer},
    limit::RequestBodyLimitLayer,
};

use super::rmcp_server::{allowed_origins, streamable_http_config, streamable_http_service};
use super::{build_auth_layer, AppState, AuthPolicy};

const MCP_BODY_LIMIT_BYTES: usize = 65_536;

pub fn router(state: AppState) -> Router {
    let rmcp_config = streamable_http_config(&state.config);
    let mcp_service =
        Router::new().nest_service("/mcp", streamable_http_service(state.clone(), rmcp_config));

    let resource_url = match &state.auth_policy {
        AuthPolicy::Mounted { .. } => state
            .config
            .auth
            .public_url
            .as_deref()
            .map(|u| Arc::<str>::from(format!("{}/mcp", u.trim_end_matches('/')))),
        AuthPolicy::LoopbackDev => None,
    };

    let authenticated = if let Some(layer) = build_auth_layer(
        &state.auth_policy,
        state.config.api_token.as_deref().map(Arc::<str>::from),
        resource_url,
    ) {
        mcp_service.layer(layer)
    } else {
        mcp_service
    };

    let oauth_router: Option<Router> = if let AuthPolicy::Mounted {
        auth_state: Some(ref state_arc),
    } = state.auth_policy
    {
        let auth_state = state_arc.as_ref().clone();
        let path_based_discovery = Router::new()
            .route(
                "/mcp/.well-known/oauth-authorization-server",
                get(lab_auth::metadata::authorization_server_metadata),
            )
            .route(
                "/mcp/.well-known/openid-configuration",
                get(lab_auth::metadata::authorization_server_metadata),
            )
            .route(
                "/mcp/.well-known/oauth-protected-resource",
                get(lab_auth::metadata::protected_resource_metadata),
            )
            .with_state(auth_state.clone());
        Some(lab_auth::routes::router(auth_state).merge(path_based_discovery))
    } else {
        None
    };

    let base: Router<()> = Router::new()
        .merge(authenticated)
        .route("/health", get(health))
        .with_state(state.clone());

    let combined = match oauth_router {
        Some(oauth) => base.merge(oauth),
        None => base,
    };

    combined
        .fallback(|| async { (StatusCode::NOT_FOUND, Json(json!({"error": "not_found"}))) })
        .layer(RequestBodyLimitLayer::new(MCP_BODY_LIMIT_BYTES))
        .layer(cors_layer(&state.config))
}

fn cors_layer(config: &crate::config::McpConfig) -> CorsLayer {
    let origins: Vec<HeaderValue> = allowed_origins(config)
        .into_iter()
        .filter_map(|o| o.parse::<HeaderValue>().ok())
        .collect();
    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::POST, Method::GET])
        .allow_headers(Any)
}

async fn health() -> impl IntoResponse {
    Json(json!({ "status": "ok" }))
}
