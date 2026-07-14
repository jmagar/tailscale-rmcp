use anyhow::Result;
use rmcp::{transport::stdio, ServiceExt};
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

use std::sync::Arc;
use tailscale_rmcp::{
    app::TailscaleService,
    cli,
    config::Config,
    mcp::{self, AppState, AuthPolicy},
    observability::Counters,
    tailscale::TailscaleClient,
};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.as_slice() {
        [f] if matches!(f.as_str(), "--help" | "-h" | "help") => {
            print_usage();
            return Ok(());
        }
        [f] if matches!(f.as_str(), "--version" | "-V" | "version") => {
            println!("tailscale {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        _ => {}
    }

    // Load ~/.tailscale-mcp/.env (or /data/.env in a container) before any
    // Config::load so the binary works on bare metal without a process manager
    // injecting env. Non-overriding: explicit process env still wins.
    tailscale_rmcp::cli::load_dotenv();

    let stdio_mode = matches!(args.as_slice(), [c] if c == "mcp");
    let serve_mode = args.is_empty()
        || matches!(args.as_slice(), [c] if c == "serve")
        || matches!(args.as_slice(), [a, b] if a == "serve" && b == "mcp");

    let log_level = if stdio_mode || !serve_mode {
        "warn"
    } else {
        "info"
    };
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level)),
        )
        .with_writer(std::io::stderr)
        .with_target(true)
        .init();

    if serve_mode {
        serve_mcp().await
    } else if stdio_mode {
        serve_stdio_mcp().await
    } else if let Some((command, json)) = tailscale_rmcp::setup::SetupCommand::parse(&args)? {
        tailscale_rmcp::setup::run(command, json)
    } else if matches!(args.as_slice(), [c] if c == "doctor")
        || matches!(args.as_slice(), [c, _] if c == "doctor")
    {
        // Doctor runs before client construction — its whole purpose is to report
        // why the server can't start, including a missing API key.
        let json = args.iter().any(|a| a == "--json" || a == "-j");
        let config = Config::load().unwrap_or_default();
        cli::run_doctor(&config, json).await
    } else {
        run_cli(args).await
    }
}

async fn serve_mcp() -> Result<()> {
    let config = Config::load()?;
    validate_bind_security(&config)?;
    let state = build_state(config).await?;

    info!(
        bind = %state.config.bind_addr(),
        server_name = %state.config.server_name,
        auth = ?state.auth_policy,
        "tailscale-rmcp starting"
    );

    let bind = state.config.bind_addr();
    let app = mcp::router(state).layer(tower_http::trace::TraceLayer::new_for_http());
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    info!(bind = %bind, "MCP HTTP server listening");

    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

/// Refuse to bind to a non-loopback address without authentication,
/// unless TAILSCALE_NOAUTH=true is explicitly set (e.g., upstream gateway handles auth).
fn validate_bind_security(config: &Config) -> Result<()> {
    let is_loopback = config.mcp.host.starts_with("127.") || config.mcp.host == "::1";
    let has_auth = (!config.mcp.no_auth && config.mcp.api_token.is_some())
        || config.mcp.auth.mode == tailscale_rmcp::config::AuthMode::OAuth;
    let noauth_override = std::env::var("TAILSCALE_NOAUTH")
        .map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes"))
        .unwrap_or(false);

    if !is_loopback && !has_auth && !noauth_override {
        anyhow::bail!(
            "Refusing to bind MCP server to {} without authentication.\n\
             Set TAILSCALE_MCP_TOKEN, use auth_mode=oauth in config.toml, or set \
             TAILSCALE_NOAUTH=true if an upstream gateway handles authentication.",
            config.mcp.host
        );
    }
    Ok(())
}

async fn serve_stdio_mcp() -> Result<()> {
    // Stdio is always LoopbackDev — trusted local pipe, no HTTP auth context.
    let config = Config::load()?;
    let client = TailscaleClient::new(&config.tailscale)?;
    let service = TailscaleService::new(client, config.tailscale.allow_destructive);
    let state = AppState {
        config: config.mcp,
        auth_policy: AuthPolicy::LoopbackDev,
        service,
        counters: Arc::new(tailscale_rmcp::observability::Counters::new()),
    };
    let svc = mcp::rmcp_server(state).serve(stdio()).await?;
    svc.waiting().await?;
    Ok(())
}

async fn run_cli(args: Vec<String>) -> Result<()> {
    let config = Config::load()?;
    let client = TailscaleClient::new(&config.tailscale)?;
    let service = TailscaleService::new(client, config.tailscale.allow_destructive);
    let (cmd, json) = cli::CliCommand::parse(&args)?;
    cli::run(&service, cmd, json).await
}

async fn build_state(config: Config) -> Result<AppState> {
    let auth_policy = resolve_auth_policy(&config).await?;
    let client = TailscaleClient::new(&config.tailscale)?;
    let service = TailscaleService::new(client, config.tailscale.allow_destructive);
    Ok(AppState {
        config: config.mcp,
        auth_policy,
        service,
        counters: Arc::new(Counters::new()),
    })
}

async fn resolve_auth_policy(config: &Config) -> Result<AuthPolicy> {
    use std::sync::Arc;
    use tailscale_rmcp::config::AuthMode;

    // Loopback or explicit no_auth → no authentication required
    if config.mcp.no_auth || config.mcp.host.starts_with("127.") {
        return Ok(AuthPolicy::LoopbackDev);
    }

    // OAuth mode: build a full AuthState
    if config.mcp.auth.mode == AuthMode::OAuth {
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
            .enable_dynamic_registration(true)
            .disable_static_token_with_oauth(config.mcp.auth.disable_static_token_with_oauth)
            .build_from_sources(auth_source_vars(config))
            .map_err(|e| anyhow::anyhow!("auth config error: {e}"))?;

        let auth_state = lab_auth::state::AuthState::new(auth_config)
            .await
            .map_err(|e| anyhow::anyhow!("auth state init failed: {e}"))?;

        return Ok(AuthPolicy::Mounted {
            auth_state: Some(Arc::new(auth_state)),
        });
    }

    // Bearer token mode (default)
    Ok(AuthPolicy::Mounted { auth_state: None })
}

fn auth_source_vars(config: &Config) -> Vec<(String, String)> {
    let auth = &config.mcp.auth;
    let mut vars = vec![
        (
            "TAILSCALE_MCP_AUTH_MODE".to_string(),
            match auth.mode {
                tailscale_rmcp::config::AuthMode::Bearer => "bearer",
                tailscale_rmcp::config::AuthMode::OAuth => "oauth",
            }
            .to_string(),
        ),
        (
            "TAILSCALE_MCP_AUTH_ADMIN_EMAIL".to_string(),
            auth.admin_email.clone(),
        ),
        (
            "TAILSCALE_MCP_AUTH_SQLITE_PATH".to_string(),
            auth.sqlite_path.clone(),
        ),
        (
            "TAILSCALE_MCP_AUTH_KEY_PATH".to_string(),
            auth.key_path.clone(),
        ),
        (
            "TAILSCALE_MCP_AUTH_ACCESS_TOKEN_TTL_SECS".to_string(),
            auth.access_token_ttl_secs.to_string(),
        ),
        (
            "TAILSCALE_MCP_AUTH_REFRESH_TOKEN_TTL_SECS".to_string(),
            auth.refresh_token_ttl_secs.to_string(),
        ),
        (
            "TAILSCALE_MCP_AUTH_CODE_TTL_SECS".to_string(),
            auth.auth_code_ttl_secs.to_string(),
        ),
        (
            "TAILSCALE_MCP_AUTH_REGISTER_REQUESTS_PER_MINUTE".to_string(),
            auth.register_rpm.to_string(),
        ),
        (
            "TAILSCALE_MCP_AUTH_AUTHORIZE_REQUESTS_PER_MINUTE".to_string(),
            auth.authorize_rpm.to_string(),
        ),
    ];
    push_optional_var(
        &mut vars,
        "TAILSCALE_MCP_PUBLIC_URL",
        auth.public_url.as_deref(),
    );
    push_optional_var(
        &mut vars,
        "TAILSCALE_MCP_GOOGLE_CLIENT_ID",
        auth.google_client_id.as_deref(),
    );
    push_optional_var(
        &mut vars,
        "TAILSCALE_MCP_GOOGLE_CLIENT_SECRET",
        auth.google_client_secret.as_deref(),
    );
    push_list_var(
        &mut vars,
        "TAILSCALE_MCP_AUTH_ALLOWED_REDIRECT_URIS",
        &auth.allowed_client_redirect_uris,
    );

    // Real environment wins over config.toml-derived defaults.
    vars.extend(std::env::vars());
    vars
}

fn push_optional_var(vars: &mut Vec<(String, String)>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        vars.push((key.to_string(), value.to_string()));
    }
}

fn push_list_var(vars: &mut Vec<(String, String)>, key: &str, values: &[String]) {
    if !values.is_empty() {
        vars.push((key.to_string(), values.join(",")));
    }
}

fn print_usage() {
    eprintln!(
        "Usage:
  rtailscale [serve]                     Start MCP HTTP server (port 40040)
  rtailscale mcp                         Start MCP stdio transport
  rtailscale doctor [--json]             Validate environment before starting
  rtailscale setup check [--json]        Check local plugin setup
  rtailscale setup repair [--json]       Repair local plugin setup
  rtailscale setup plugin-hook [--no-repair] [--json]

Read:
  rtailscale devices [--json]            All devices in the tailnet
  rtailscale device <id> [--json]        Single device details
  rtailscale routes <device-id> [--json] Subnet routes for a device
  rtailscale keys [--json]               API keys in the tailnet
  rtailscale acl [--json]                ACL policy
  rtailscale dns [--json]                DNS nameservers, search paths, preferences
  rtailscale users [--json]              Users in the tailnet

Write:
  rtailscale authorize <device-id> [--json]                    Authorize a device

Destructive (requires TAILSCALE_ALLOW_DESTRUCTIVE=true):
  rtailscale delete-device <device-id> --confirm [--json]      Delete a device

NOTE: This repo ships the binary name 'rtailscale' to avoid conflicting with
  the official Tailscale CLI named 'tailscale'.

Environment:
  TAILSCALE_API_KEY                 Tailscale API key (required)
  TAILSCALE_TAILNET                 Tailnet: org domain or '-' for personal (default: -)
  TAILSCALE_ALLOW_DESTRUCTIVE       Enable delete-device (default: false)
  TAILSCALE_MCP_HOST                Bind host (default: 0.0.0.0)
  TAILSCALE_MCP_PORT                Bind port (default: 40040)
  TAILSCALE_MCP_NO_AUTH             Disable auth (loopback only)
  TAILSCALE_MCP_TOKEN               Static bearer token
  TAILSCALE_MCP_AUTH_MODE           'bearer' (default) or 'oauth'
  TAILSCALE_MCP_PUBLIC_URL          Public URL for OAuth discovery
  TAILSCALE_MCP_GOOGLE_CLIENT_ID    Google OAuth client ID
  TAILSCALE_MCP_GOOGLE_CLIENT_SECRET Google OAuth client secret
  TAILSCALE_MCP_AUTH_ADMIN_EMAIL    Admin email for OAuth
  RUST_LOG                          Log filter"
    );
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!(error = %e, "CTRL+C handler failed");
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(e) => {
                tracing::error!(error = %e, "SIGTERM handler failed");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
    tracing::info!("Shutdown signal received");
}
