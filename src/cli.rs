use std::{
    net::TcpListener,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use anyhow::{bail, Result};
use serde::Serialize;
use serde_json::Value;

use crate::{app::TailscaleService, config::Config};

/// Parsed CLI subcommand.
pub enum CliCommand {
    Devices,
    Device { id: String },
    Routes { device_id: String },
    Keys,
    Acl,
    Dns,
    Users,
    Authorize { device_id: String },
    DeleteDevice { device_id: String, confirm: bool },
    Doctor,
}

impl CliCommand {
    /// Parse args into a command, returning (command, json_flag).
    pub fn parse(args: &[String]) -> Result<(Self, bool)> {
        let (args, json) = strip_json_flag(args);
        let cmd = match args.as_slice() {
            [c] if c == "devices" => CliCommand::Devices,
            [c, id] if c == "device" => CliCommand::Device { id: id.clone() },
            [c, id] if c == "routes" => CliCommand::Routes {
                device_id: id.clone(),
            },
            [c] if c == "keys" => CliCommand::Keys,
            [c] if c == "acl" => CliCommand::Acl,
            [c] if c == "dns" => CliCommand::Dns,
            [c] if c == "users" => CliCommand::Users,
            [c, id] if c == "authorize" => CliCommand::Authorize {
                device_id: id.clone(),
            },
            [c, id] if c == "delete-device" => CliCommand::DeleteDevice {
                device_id: id.clone(),
                confirm: false,
            },
            [c, id, flag] if c == "delete-device" && flag == "--confirm" => {
                CliCommand::DeleteDevice {
                    device_id: id.clone(),
                    confirm: true,
                }
            }
            [c] if c == "doctor" => CliCommand::Doctor,
            _ => bail!(
                "unknown command; run with --help for usage\ngot: {:?}",
                args
            ),
        };
        Ok((cmd, json))
    }
}

/// Execute a CLI command, printing the result to stdout.
pub async fn run(service: &TailscaleService, cmd: CliCommand, json: bool) -> Result<()> {
    let result = match cmd {
        CliCommand::Devices => service.devices().await?,
        CliCommand::Device { id } => service.device(&id).await?,
        CliCommand::Routes { device_id } => service.device_routes(&device_id).await?,
        CliCommand::Keys => service.keys().await?,
        CliCommand::Acl => service.acl().await?,
        CliCommand::Dns => service.dns().await?,
        CliCommand::Users => service.users().await?,
        CliCommand::Authorize { device_id } => service.authorize_device(&device_id).await?,
        CliCommand::DeleteDevice { device_id, confirm } => {
            service.delete_device(&device_id, confirm).await?
        }
        CliCommand::Doctor => unreachable!("doctor handled in main before service construction"),
    };

    print_result(&result, json);
    Ok(())
}

fn print_result(value: &Value, _json: bool) {
    // Both branches use pretty JSON for now; a future table formatter can branch on `_json`.
    println!(
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_default()
    );
}

/// Strip `--json` / `-j` flags and return remaining args + whether flag was present.
fn strip_json_flag(args: &[String]) -> (Vec<String>, bool) {
    let mut json = false;
    let remaining: Vec<String> = args
        .iter()
        .filter(|a| {
            if a.as_str() == "--json" || a.as_str() == "-j" {
                json = true;
                false
            } else {
                true
            }
        })
        .cloned()
        .collect();
    (remaining, json)
}

// ── doctor ─────────────────────────────────────────────────────────────────────

/// Returns the local data directory for rustscale.
/// Uses `~/.tailscale-mcp/` (not `~/.tailscale`) to avoid conflict with the
/// real Tailscale client data directory.
pub fn default_data_dir() -> PathBuf {
    if std::path::Path::new("/.dockerenv").exists()
        || std::env::var("RUNNING_IN_CONTAINER").is_ok()
        || std::env::var("container").is_ok()
    {
        return PathBuf::from("/data");
    }
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".tailscale-mcp")
}

/// Load `~/.tailscale-mcp/.env` (or `/data/.env` in a container) into the process
/// environment if present.
///
/// Best-effort: a missing file is ignored, and existing env vars are NOT
/// overridden — values injected by docker-compose/systemd or the plugin hook's
/// `CLAUDE_PLUGIN_OPTION_*` mapping still take precedence. Lets the binary find
/// its credentials directly from `~/.tailscale-mcp/.env` without a process
/// manager. Call once at startup before `Config::load`. A symlinked `.env` is
/// refused (the dir holds secrets; mirrors axon).
pub fn load_dotenv() {
    let env_path = default_data_dir().join(".env");
    match std::fs::symlink_metadata(&env_path) {
        Ok(md) if md.file_type().is_symlink() => {
            eprintln!(
                "error: refusing to load symlinked .env at {} (potential symlink attack)",
                env_path.display()
            );
            std::process::exit(1);
        }
        Ok(_) => {
            let _ = dotenvy::from_path(&env_path);
        }
        Err(_) => {}
    }
}

#[derive(Serialize)]
pub struct DoctorCheck {
    pub category: &'static str,
    pub name: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
}

impl DoctorCheck {
    fn pass(category: &'static str, name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            category,
            name: name.into(),
            ok: true,
            value: Some(value.into()),
            hint: None,
            latency_ms: None,
        }
    }

    fn fail(category: &'static str, name: impl Into<String>, hint: impl Into<String>) -> Self {
        Self {
            category,
            name: name.into(),
            ok: false,
            value: None,
            hint: Some(hint.into()),
            latency_ms: None,
        }
    }
}

// ── individual checks ─────────────────────────────────────────────────────────

fn check_config_file(data_dir: &Path) -> DoctorCheck {
    let p = data_dir.join("config.toml");
    if p.exists() {
        DoctorCheck::pass("config", "Config file", p.display().to_string())
    } else {
        DoctorCheck::fail(
            "config",
            "Config file",
            format!("{} not found — create it or rely on env vars", p.display()),
        )
    }
}

fn check_dir_writable(category: &'static str, label: &str, dir: &Path) -> DoctorCheck {
    match std::fs::create_dir_all(dir) {
        Ok(_) => {}
        Err(e) => {
            return DoctorCheck::fail(
                category,
                label.to_string(),
                format!("cannot create {}: {}", dir.display(), e),
            );
        }
    }

    // Check writability by probing a temp file.
    let probe = dir.join(".doctor_write_probe");
    match std::fs::write(&probe, b"") {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            // Report directory size if available.
            let size_str = dir_size_human(dir);
            DoctorCheck::pass(
                category,
                label.to_string(),
                format!("{} (writable{})", dir.display(), size_str),
            )
        }
        Err(e) => DoctorCheck::fail(
            category,
            label.to_string(),
            format!("{} is not writable: {}", dir.display(), e),
        ),
    }
}

fn dir_size_human(dir: &Path) -> String {
    // Best-effort directory size via du. Silently returns empty on failure.
    let output = std::process::Command::new("du")
        .args(["-sh", "--"])
        .arg(dir)
        .output();
    match output {
        Ok(o) if o.status.success() => {
            let s = String::from_utf8_lossy(&o.stdout);
            let size = s.split_whitespace().next().unwrap_or("").to_string();
            if size.is_empty() {
                String::new()
            } else {
                format!(", {size}")
            }
        }
        _ => String::new(),
    }
}

fn check_binary_in_path() -> Vec<DoctorCheck> {
    let mut checks = Vec::new();
    let install_path = std::env::var("HOME")
        .map(|h| format!("{h}/.local/bin/tailscale"))
        .unwrap_or_default();

    // Find where `tailscale` resolves to in PATH.
    let which_output = std::process::Command::new("which")
        .arg("tailscale")
        .output();

    match which_output {
        Ok(o) if o.status.success() => {
            let resolved = String::from_utf8_lossy(&o.stdout).trim().to_string();
            checks.push(DoctorCheck::pass(
                "config",
                "Binary in PATH",
                resolved.clone(),
            ));

            // Binary conflict check: if it resolves to something other than our
            // install location, the real tailscale CLI may be in front of us.
            if !resolved.contains("tailscale-mcp") && resolved != install_path {
                // Check if this is the real Tailscale CLI by running `tailscale version`.
                let is_real = std::process::Command::new(&resolved)
                    .arg("version")
                    .output()
                    .map(|o| {
                        let out = String::from_utf8_lossy(&o.stdout);
                        // Real Tailscale CLI prints a version like "1.xx.x"
                        out.contains('.') && !out.contains("rustscale")
                    })
                    .unwrap_or(false);

                if is_real {
                    checks.push(DoctorCheck {
                        category: "config",
                        name: "Binary name conflict".to_string(),
                        ok: false,
                        value: Some(resolved),
                        hint: Some(
                            "The real Tailscale CLI binary is in PATH as 'tailscale' and may \
                             shadow or be shadowed by this binary. Consider installing rustscale \
                             as 'tailscale-mcp' instead (set BINARY_NAME=tailscale-mcp when \
                             re-running install.sh)."
                                .to_string(),
                        ),
                        latency_ms: None,
                    });
                }
            }
        }
        _ => {
            checks.push(DoctorCheck::fail(
                "config",
                "Binary in PATH",
                format!(
                    "'tailscale' not found in PATH — install to {install_path} and ensure \
                     ~/.local/bin is in your PATH"
                ),
            ));
        }
    }
    checks
}

fn check_required_var(name: &'static str, value: &str) -> DoctorCheck {
    if value.is_empty() {
        DoctorCheck::fail(
            "credentials",
            name,
            format!("not set — set {name} in ~/.tailscale-mcp/.env or your environment"),
        )
    } else {
        DoctorCheck::pass("credentials", name, "set")
    }
}

fn check_optional_var_default(name: &'static str, value: &str, default_desc: &str) -> DoctorCheck {
    if value.is_empty() || value == "-" {
        DoctorCheck::pass(
            "credentials",
            name,
            format!("using default ({default_desc})"),
        )
    } else {
        DoctorCheck::pass("credentials", name, value.to_string())
    }
}

async fn check_upstream(api_key: &str, tailnet: &str) -> DoctorCheck {
    let url = format!("https://api.tailscale.com/api/v2/tailnet/{tailnet}/devices");
    let url_for_display = url.clone();
    let start = Instant::now();
    let result = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .ok()
        .map(|c| async move { c.get(&url).bearer_auth(api_key).send().await });

    let (status_str, ok, latency_ms) = match result {
        None => ("failed to build HTTP client".to_string(), false, None),
        Some(fut) => match fut.await {
            Ok(resp) => {
                let ms = start.elapsed().as_millis() as u64;
                let s = resp.status();
                let ok = s.is_success();
                (format!("{s}"), ok, Some(ms))
            }
            Err(e) => {
                let ms = start.elapsed().as_millis() as u64;
                (format!("error: {e}"), false, Some(ms))
            }
        },
    };

    DoctorCheck {
        category: "connectivity",
        name: "Upstream reachable".to_string(),
        ok,
        value: Some(format!("{url_for_display} → {status_str}")),
        hint: if ok {
            None
        } else {
            Some(format!(
                "Could not reach {url_for_display} — check TAILSCALE_API_KEY and network access"
            ))
        },
        latency_ms,
    }
}

fn check_port_available(port: u16) -> DoctorCheck {
    match TcpListener::bind(("0.0.0.0", port)) {
        Ok(_) => DoctorCheck::pass("mcp_server", format!("MCP port {port}"), "available"),
        Err(e) => DoctorCheck::fail(
            "mcp_server",
            format!("MCP port {port}"),
            format!("port {port} is already in use ({e}) — change TAILSCALE_MCP_PORT"),
        ),
    }
}

fn check_destructive_warn(allow: bool) -> Option<DoctorCheck> {
    if allow {
        Some(DoctorCheck {
            category: "mcp_server",
            name: "TAILSCALE_ALLOW_DESTRUCTIVE".to_string(),
            ok: true,
            value: Some(
                "true — WARNING: destructive operations (delete_device) are enabled!".to_string(),
            ),
            hint: Some(
                "Set TAILSCALE_ALLOW_DESTRUCTIVE=false if you do not need device deletion."
                    .to_string(),
            ),
            latency_ms: None,
        })
    } else {
        None
    }
}

// ── report printing ───────────────────────────────────────────────────────────

fn print_doctor_report(checks: &[DoctorCheck]) {
    eprintln!();
    eprintln!(
        "rustscale v{} — environment check",
        env!("CARGO_PKG_VERSION")
    );
    eprintln!();

    let categories: &[(&str, &str)] = &[
        ("config", "Config"),
        ("credentials", "Service credentials"),
        ("connectivity", "Connectivity"),
        ("mcp_server", "MCP server"),
    ];

    for (cat_key, cat_label) in categories {
        let cat_checks: Vec<&DoctorCheck> =
            checks.iter().filter(|c| c.category == *cat_key).collect();
        if cat_checks.is_empty() {
            continue;
        }

        eprintln!("  {cat_label}");
        eprintln!("  {}", "─".repeat(44));

        for check in &cat_checks {
            let icon = if check.ok { "✓" } else { "✗" };
            let name_padded = format!("{:<24}", format!("{}:", check.name));

            if let Some(ref val) = check.value {
                eprintln!("  {icon} {name_padded} {val}");
            } else {
                eprintln!("  {icon} {name_padded} (not found)");
            }

            if let Some(ref hint) = check.hint {
                eprintln!("    → {hint}");
            }

            if let Some(ms) = check.latency_ms {
                if check.ok {
                    eprintln!("    ({ms} ms)");
                }
            }
        }
        eprintln!();
    }
}

// ── public entry point ────────────────────────────────────────────────────────

/// Run all pre-flight checks and report results.
///
/// Exit code: 0 = all checks pass, 1 = one or more failures.
pub async fn run_doctor(config: &Config, json: bool) -> Result<()> {
    let mut checks: Vec<DoctorCheck> = Vec::new();

    let data_dir = default_data_dir();

    // ── Config ────────────────────────────────────────────────────────────────
    checks.push(check_config_file(&data_dir));
    checks.push(check_dir_writable("config", "Data directory", &data_dir));
    checks.push(check_dir_writable(
        "config",
        "Log directory",
        &data_dir.join("logs"),
    ));
    checks.extend(check_binary_in_path());

    // ── Credentials ───────────────────────────────────────────────────────────
    checks.push(check_required_var(
        "TAILSCALE_API_KEY",
        &config.tailscale.api_key,
    ));
    checks.push(check_optional_var_default(
        "TAILSCALE_TAILNET",
        &config.tailscale.tailnet,
        "personal account",
    ));

    // ── Connectivity (skipped if API key missing) ─────────────────────────────
    if !config.tailscale.api_key.is_empty() {
        checks.push(check_upstream(&config.tailscale.api_key, &config.tailscale.tailnet).await);
    }

    // ── MCP server ────────────────────────────────────────────────────────────
    checks.push(check_port_available(config.mcp.port));

    if let Some(warn) = check_destructive_warn(config.tailscale.allow_destructive) {
        checks.push(warn);
    }

    // ── Report ────────────────────────────────────────────────────────────────
    let failures = checks.iter().filter(|c| !c.ok).count();

    if json {
        println!("{}", serde_json::to_string_pretty(&checks)?);
    } else {
        print_doctor_report(&checks);
        eprintln!("  {}", "━".repeat(44));
        if failures == 0 {
            eprintln!("  All checks passed. Ready to run: tailscale serve");
        } else {
            eprintln!(
                "  {failures} issue{} found. Fix {} before running: tailscale serve",
                if failures == 1 { "" } else { "s" },
                if failures == 1 { "it" } else { "them" }
            );
        }
        eprintln!();
    }

    if failures > 0 {
        std::process::exit(1);
    }
    Ok(())
}
