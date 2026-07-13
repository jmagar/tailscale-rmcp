use std::net::TcpListener;
use std::path::PathBuf;

use anyhow::{bail, Result};
use serde::Serialize;

const BINARY_NAME: &str = "rtailscale";
const APPDATA_ENV: &str = "TAILSCALE_MCP_HOME";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupCommand {
    Check,
    Repair,
    /// Copy this binary into ~/.local/bin so it is callable as a bare command
    /// in the user's own terminal, independent of Claude Code.
    Install,
    PluginHook {
        no_repair: bool,
    },
}

impl SetupCommand {
    pub fn parse(args: &[String]) -> Result<Option<(Self, bool)>> {
        let json = args.iter().any(|arg| arg == "--json");
        let rest: Vec<&str> = args
            .iter()
            .filter(|arg| arg.as_str() != "--json")
            .map(String::as_str)
            .collect();

        let command = match rest.as_slice() {
            ["setup", "check"] => Self::Check,
            ["setup", "repair"] => Self::Repair,
            ["setup", "install"] => Self::Install,
            ["setup", "plugin-hook"] => Self::PluginHook { no_repair: false },
            ["setup", "plugin-hook", "--no-repair"] => Self::PluginHook { no_repair: true },
            ["setup", ..] => bail!("unknown setup command: {}", rest.join(" ")),
            _ => return Ok(None),
        };
        Ok(Some((command, json)))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SetupCheck {
    pub name: &'static str,
    pub ok: bool,
    pub severity: SetupSeverity,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SetupReport {
    pub mode: &'static str,
    pub appdata_dir: PathBuf,
    pub env_path: PathBuf,
    pub checks: Vec<SetupCheck>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PluginHookReport {
    pub exit_policy: ExitPolicy,
    pub ran_repair: bool,
    pub no_repair: bool,
    pub blocking_failures: Vec<String>,
    pub advisory_failures: Vec<String>,
    pub check: SetupReport,
    pub repair: Option<SetupReport>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SetupSeverity {
    Blocking,
    Advisory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExitPolicy {
    Success,
    AdvisoryFailure,
    BlockingFailure,
}

pub fn run(command: SetupCommand, json: bool) -> Result<()> {
    match command {
        SetupCommand::Check => {
            let report = check_report();
            print_setup_report(&report, json)?;
            fail_if_setup_failed(&report)
        }
        SetupCommand::Repair => {
            let report = repair_report()?;
            print_setup_report(&report, json)?;
            fail_if_setup_failed(&report)
        }
        SetupCommand::Install => {
            let dest = install_self()?;
            println!("installed {BINARY_NAME} -> {}", dest.display());
            Ok(())
        }
        SetupCommand::PluginHook { no_repair } => {
            let report = plugin_hook_report(no_repair)?;
            print_plugin_hook_report(&report, json)?;
            if matches!(report.exit_policy, ExitPolicy::BlockingFailure) {
                bail!(
                    "{BINARY_NAME} setup plugin-hook completed with blocking failures: {}",
                    report.blocking_failures.join(", ")
                );
            }
            Ok(())
        }
    }
}

/// Copy the running binary into `~/.local/bin/<name>` so it is callable as a
/// bare command in the user's own terminal, independent of Claude Code.
///
/// Uses the running executable's own file name as the destination, so this is
/// identical across every server repo. Copy (not symlink) so it survives
/// `/plugin update`, which changes the plugin cache path a symlink would dangle
/// to. Atomic via temp + rename; idempotent; depends only on std + anyhow.
fn install_self() -> Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let name = exe
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("cannot determine binary name from {}", exe.display()))?;
    let home = std::env::var_os("HOME").ok_or_else(|| anyhow::anyhow!("HOME is not set"))?;
    let bin_dir = PathBuf::from(home).join(".local").join("bin");
    std::fs::create_dir_all(&bin_dir)?;
    let dest = bin_dir.join(name);

    // Running the already-installed copy: nothing to do.
    if dest == exe {
        return Ok(dest);
    }

    let tmp = bin_dir.join(format!(".{}.tmp", name.to_string_lossy()));
    std::fs::copy(&exe, &tmp)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))?;
    }
    std::fs::rename(&tmp, &dest).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })?;

    let on_path = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|d| d == bin_dir))
        .unwrap_or(false);
    if !on_path {
        eprintln!(
            "note: {} is not on your PATH; add:  export PATH=\"$HOME/.local/bin:$PATH\"",
            bin_dir.display()
        );
    }
    Ok(dest)
}

/// Translate Claude Code plugin options (`CLAUDE_PLUGIN_OPTION_*`) into the
/// `TAILSCALE_*` process env vars the binary reads, plus the appdata HOME var.
///
/// This replaces the former `plugin-setup.sh` wrapper: the binary now owns the
/// env-var mapping itself, so the plugin hook calls the binary directly. The
/// setup checks (`appdata_dir`, `port_check`, `binary_check`) read these live
/// from `std::env`, so mapping here — before `check_report()` — is what makes
/// them effective. Values containing newlines/CR are skipped, mirroring the
/// script's `reject_unsafe_value` guard.
///
/// The script's `.env`-fallback (re-exporting a previously persisted value when
/// an option was unset) is intentionally dropped: the binary never persists
/// option values to `.env` (repair only writes a header comment) and the setup
/// checks never read that file, so the fallback was immaterial to the hook
/// contract. The server receives option values via `.mcp.json`'s
/// `${user_config.*}` block, not the hook's process env.
fn apply_plugin_options() {
    // CLAUDE_PLUGIN_DATA is intentionally NOT mapped to TAILSCALE_MCP_HOME: appdata
    // resolves to the canonical ~/.tailscale-mcp/ (the same place the binary loads
    // .env from via cli::load_dotenv), not the plugin's sandboxed data dir. An
    // explicit TAILSCALE_MCP_HOME override is still honored by appdata_dir().

    // CLAUDE_PLUGIN_OPTION_<OPT> -> <TAILSCALE_ENVVAR>
    let map = [
        ("CLAUDE_PLUGIN_OPTION_API_TOKEN", "TAILSCALE_MCP_TOKEN"),
        ("CLAUDE_PLUGIN_OPTION_NO_AUTH", "TAILSCALE_MCP_NO_AUTH"),
        ("CLAUDE_PLUGIN_OPTION_MCP_HOST", "TAILSCALE_MCP_HOST"),
        ("CLAUDE_PLUGIN_OPTION_MCP_PORT", "TAILSCALE_MCP_PORT"),
        ("CLAUDE_PLUGIN_OPTION_AUTH_MODE", "TAILSCALE_MCP_AUTH_MODE"),
        (
            "CLAUDE_PLUGIN_OPTION_PUBLIC_URL",
            "TAILSCALE_MCP_PUBLIC_URL",
        ),
        (
            "CLAUDE_PLUGIN_OPTION_GOOGLE_CLIENT_ID",
            "TAILSCALE_MCP_GOOGLE_CLIENT_ID",
        ),
        (
            "CLAUDE_PLUGIN_OPTION_GOOGLE_CLIENT_SECRET",
            "TAILSCALE_MCP_GOOGLE_CLIENT_SECRET",
        ),
        (
            "CLAUDE_PLUGIN_OPTION_AUTH_ADMIN_EMAIL",
            "TAILSCALE_MCP_AUTH_ADMIN_EMAIL",
        ),
        (
            "CLAUDE_PLUGIN_OPTION_TAILSCALE_API_KEY",
            "TAILSCALE_API_KEY",
        ),
        (
            "CLAUDE_PLUGIN_OPTION_TAILSCALE_TAILNET",
            "TAILSCALE_TAILNET",
        ),
        (
            "CLAUDE_PLUGIN_OPTION_ALLOW_DESTRUCTIVE",
            "TAILSCALE_ALLOW_DESTRUCTIVE",
        ),
    ];
    for (opt, dest) in map {
        if let Some(v) = std::env::var_os(opt) {
            let s = v.to_string_lossy();
            if s.is_empty() || s.contains('\n') || s.contains('\r') {
                continue;
            }
            // edition 2021: set_var is safe (no unsafe block required).
            std::env::set_var(dest, v);
        }
    }
}

fn plugin_hook_report(no_repair: bool) -> Result<PluginHookReport> {
    // Translate CLAUDE_PLUGIN_OPTION_* into TAILSCALE_* env vars before any setup
    // check reads them. Replaces the deleted plugin-setup.sh wrapper.
    apply_plugin_options();
    // Keep the user's terminal copy in ~/.local/bin fresh each session so it
    // survives `/plugin update`. Best-effort: never fail the hook over it.
    if let Err(e) = install_self() {
        eprintln!("{BINARY_NAME} setup plugin-hook: self-install skipped: {e}");
    }
    let check = check_report();
    let repair = if no_repair || setup_ok(&check) {
        None
    } else {
        Some(repair_report()?)
    };
    let active = repair.as_ref().unwrap_or(&check);
    let blocking_failures = blocking_failures(active);
    let advisory_failures = advisory_failures(active);
    Ok(PluginHookReport {
        exit_policy: if !blocking_failures.is_empty() {
            ExitPolicy::BlockingFailure
        } else if !advisory_failures.is_empty() {
            ExitPolicy::AdvisoryFailure
        } else {
            ExitPolicy::Success
        },
        ran_repair: repair.is_some(),
        no_repair,
        blocking_failures,
        advisory_failures,
        check,
        repair,
    })
}

fn check_report() -> SetupReport {
    let appdata_dir = appdata_dir();
    let env_path = appdata_dir.join(".env");
    SetupReport {
        mode: "check",
        appdata_dir,
        env_path: env_path.clone(),
        checks: vec![
            SetupCheck {
                name: "appdata_dir",
                ok: env_path.parent().is_some_and(|path| path.is_dir()),
                severity: SetupSeverity::Blocking,
                detail: env_path
                    .parent()
                    .map(|path| path.display().to_string())
                    .unwrap_or_default(),
            },
            SetupCheck {
                name: "env_file",
                ok: env_path.is_file(),
                severity: SetupSeverity::Advisory,
                detail: env_path.display().to_string(),
            },
            binary_check(),
            port_check(),
        ],
    }
}

fn repair_report() -> Result<SetupReport> {
    let dir = appdata_dir();
    std::fs::create_dir_all(&dir)?;
    let env_path = dir.join(".env");
    if !env_path.exists() {
        std::fs::write(&env_path, b"# Managed by tailscale setup repair.\n")?;
    }
    Ok(SetupReport {
        mode: "repair",
        ..check_report()
    })
}

fn binary_check() -> SetupCheck {
    match find_binary(BINARY_NAME) {
        Some(path) => SetupCheck {
            name: "binary",
            ok: true,
            severity: SetupSeverity::Blocking,
            detail: path.display().to_string(),
        },
        None => SetupCheck {
            name: "binary",
            ok: false,
            severity: SetupSeverity::Blocking,
            detail: format!("{BINARY_NAME} not found in PATH"),
        },
    }
}

fn port_check() -> SetupCheck {
    let port = setup_port("TAILSCALE_MCP_PORT", 40040);
    match TcpListener::bind(("127.0.0.1", port)) {
        Ok(_) => SetupCheck {
            name: "mcp_port",
            ok: true,
            severity: SetupSeverity::Advisory,
            detail: format!("port {port} available"),
        },
        Err(error) => SetupCheck {
            name: "mcp_port",
            ok: false,
            severity: SetupSeverity::Advisory,
            detail: format!("port {port} is already in use: {error}"),
        },
    }
}

fn setup_port(env_name: &str, default: u16) -> u16 {
    std::env::var(env_name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn find_binary(binary: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path)
            .map(|dir| dir.join(binary))
            .find(|candidate| candidate.is_file())
    })
}

fn appdata_dir() -> PathBuf {
    if let Some(value) = std::env::var_os(APPDATA_ENV) {
        return PathBuf::from(value);
    }
    crate::cli::default_data_dir()
}

fn setup_ok(report: &SetupReport) -> bool {
    blocking_failures(report).is_empty()
}

fn blocking_failures(report: &SetupReport) -> Vec<String> {
    report
        .checks
        .iter()
        .filter(|check| !check.ok && check.severity == SetupSeverity::Blocking)
        .map(|check| check.name.to_string())
        .collect()
}

fn advisory_failures(report: &SetupReport) -> Vec<String> {
    report
        .checks
        .iter()
        .filter(|check| !check.ok && check.severity == SetupSeverity::Advisory)
        .map(|check| check.name.to_string())
        .collect()
}

fn fail_if_setup_failed(report: &SetupReport) -> Result<()> {
    let failures = blocking_failures(report);
    if failures.is_empty() {
        Ok(())
    } else {
        bail!("setup {} failed: {}", report.mode, failures.join(", "))
    }
}

fn print_setup_report(report: &SetupReport, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(report)?);
    } else {
        println!("{BINARY_NAME} setup {}", report.mode);
        println!("Appdata: {}", report.appdata_dir.display());
        println!("Env: {}", report.env_path.display());
        for check in &report.checks {
            println!(
                "{}\t{}\t{}",
                if check.ok { "ok" } else { "fail" },
                check.name,
                check.detail
            );
        }
    }
    Ok(())
}

fn print_plugin_hook_report(report: &PluginHookReport, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(report)?);
    } else {
        print_setup_report(&report.check, false)?;
        if let Some(repair) = &report.repair {
            print_setup_report(repair, false)?;
        }
        println!("Plugin hook policy: {:?}", report.exit_policy);
        println!("Plugin hook ran repair: {}", report.ran_repair);
    }
    Ok(())
}
