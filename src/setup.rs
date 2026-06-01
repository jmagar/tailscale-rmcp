use std::net::TcpListener;
use std::path::PathBuf;

use anyhow::{bail, Result};
use serde::Serialize;

const BINARY_NAME: &str = "tailscale";
const APPDATA_ENV: &str = "TAILSCALE_MCP_HOME";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupCommand {
    Check,
    Repair,
    PluginHook { no_repair: bool },
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

fn plugin_hook_report(no_repair: bool) -> Result<PluginHookReport> {
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
