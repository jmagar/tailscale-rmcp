use std::sync::Arc;

use lab_auth::AuthLayer;

use crate::{app::TailscaleService, config::McpConfig, observability::Counters};

mod metadata;
mod prompts;
mod rmcp_server;
mod routes;
mod schemas;
mod tools;

pub use rmcp_server::{
    rmcp_server, streamable_http_config, streamable_http_service, TailscaleRmcpServer,
};
pub use routes::router;

/// Authentication policy attached to [`AppState`].
///
/// Intentionally an enum so constructing an `AppState` requires an explicit
/// choice — there is no `Default` impl.
#[derive(Clone)]
pub enum AuthPolicy {
    /// No authentication. Only legal when bound to a loopback address.
    /// Scope checks are bypassed — the bind itself is the trust boundary.
    LoopbackDev,
    /// Authentication middleware is mounted. Scope checks MUST run.
    /// - `Some(auth_state)`: OAuth mode (Google flow + JWKS issuance)
    /// - `None`: static bearer token only
    Mounted {
        auth_state: Option<Arc<lab_auth::state::AuthState>>,
    },
}

impl std::fmt::Debug for AuthPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthPolicy::LoopbackDev => f.write_str("AuthPolicy::LoopbackDev"),
            AuthPolicy::Mounted {
                auth_state: Some(_),
            } => f.write_str("AuthPolicy::Mounted { auth_state: Some(<AuthState>) }"),
            AuthPolicy::Mounted { auth_state: None } => {
                f.write_str("AuthPolicy::Mounted { auth_state: None /* bearer-only */ }")
            }
        }
    }
}

/// Shared application state injected into every request handler.
#[derive(Clone)]
pub struct AppState {
    pub config: McpConfig,
    pub auth_policy: AuthPolicy,
    pub service: TailscaleService,
    /// Global request/error counters. Arc-shared so all cloned handler instances
    /// increment the same totals.
    pub counters: Arc<Counters>,
}

/// Build an [`AuthLayer`] from an [`AuthPolicy`], or `None` for
/// [`AuthPolicy::LoopbackDev`] (loopback bind is the trust boundary).
pub fn build_auth_layer(
    policy: &AuthPolicy,
    static_token: Option<Arc<str>>,
    resource_url: Option<Arc<str>>,
) -> Option<AuthLayer> {
    match policy {
        AuthPolicy::LoopbackDev => None,
        AuthPolicy::Mounted { auth_state } => Some(
            AuthLayer::new()
                .with_static_token(static_token)
                .with_auth_state(auth_state.clone())
                .with_static_token_scopes(vec![
                    "tailscale:read".into(),
                    "tailscale:write".into(),
                    "tailscale:admin".into(),
                ])
                .with_resource_url(resource_url)
                .with_allow_session_cookie(false),
        ),
    }
}

// ── test support ──────────────────────────────────────────────────────────────

#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub mod testing {
    use std::sync::Arc;

    use crate::{
        app::TailscaleService,
        config::{McpConfig, TailscaleConfig},
        mcp::{AppState, AuthPolicy},
        observability::Counters,
        tailscale::TailscaleClient,
    };

    fn stub_service() -> TailscaleService {
        let client = TailscaleClient::new(&TailscaleConfig {
            api_key: "test".into(),
            tailnet: "-".into(),
            allow_destructive: false,
        })
        .expect("stub client should build");
        TailscaleService::new(client, false)
    }

    fn stub_service_destructive() -> TailscaleService {
        let client = TailscaleClient::new(&TailscaleConfig {
            api_key: "test".into(),
            tailnet: "-".into(),
            allow_destructive: true,
        })
        .expect("stub client should build");
        TailscaleService::new(client, true)
    }

    pub fn loopback_state() -> AppState {
        AppState {
            config: McpConfig::default(),
            auth_policy: AuthPolicy::LoopbackDev,
            service: stub_service(),
            counters: Arc::new(Counters::new()),
        }
    }

    pub fn bearer_state(token: &str) -> AppState {
        AppState {
            config: McpConfig {
                api_token: Some(token.to_string()),
                ..McpConfig::default()
            },
            auth_policy: AuthPolicy::Mounted { auth_state: None },
            service: stub_service(),
            counters: Arc::new(Counters::new()),
        }
    }

    pub fn destructive_state() -> AppState {
        AppState {
            config: McpConfig::default(),
            auth_policy: AuthPolicy::LoopbackDev,
            service: stub_service_destructive(),
            counters: Arc::new(Counters::new()),
        }
    }

    pub async fn oauth_state(data_dir: &std::path::Path) -> AppState {
        let auth_state = build_auth_state(data_dir).await;
        AppState {
            config: McpConfig {
                auth: crate::config::AuthConfig {
                    public_url: Some("https://tailscale.example.com".to_string()),
                    ..Default::default()
                },
                ..McpConfig::default()
            },
            auth_policy: AuthPolicy::Mounted {
                auth_state: Some(Arc::new(auth_state)),
            },
            service: stub_service(),
            counters: Arc::new(Counters::new()),
        }
    }

    pub async fn build_auth_state(data_dir: &std::path::Path) -> lab_auth::state::AuthState {
        let vars: Vec<(String, String)> = vec![
            ("TAILSCALE_MCP_AUTH_MODE".into(), "oauth".into()),
            (
                "TAILSCALE_MCP_PUBLIC_URL".into(),
                "https://tailscale.example.com".into(),
            ),
            (
                "TAILSCALE_MCP_GOOGLE_CLIENT_ID".into(),
                "test-client-id".into(),
            ),
            (
                "TAILSCALE_MCP_GOOGLE_CLIENT_SECRET".into(),
                "test-client-secret".into(),
            ),
            (
                "TAILSCALE_MCP_AUTH_ADMIN_EMAIL".into(),
                "admin@example.com".into(),
            ),
            (
                "TAILSCALE_MCP_AUTH_SQLITE_PATH".into(),
                data_dir.join("auth.db").to_str().unwrap().into(),
            ),
            (
                "TAILSCALE_MCP_AUTH_KEY_PATH".into(),
                data_dir.join("auth-jwt.pem").to_str().unwrap().into(),
            ),
        ];

        let auth_config = lab_auth::config::AuthConfigBuilder::new()
            .env_prefix("TAILSCALE_MCP")
            .session_cookie_name("tailscale_mcp_session")
            .scopes_supported(vec![
                "tailscale:read".into(),
                "tailscale:write".into(),
                "tailscale:admin".into(),
            ])
            .default_scope("tailscale:read")
            .resource_path("/mcp")
            .build_from_sources(vars)
            .expect("test auth config should build");

        lab_auth::state::AuthState::new(auth_config)
            .await
            .expect("test auth state should init")
    }

    /// Drive the tool dispatch directly without HTTP.
    /// Re-exports the internal `execute_tool` for integration tests.
    pub async fn call_tool(
        state: &AppState,
        tool: &str,
        args: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        super::tools::execute_tool(state, tool, args).await
    }
}
