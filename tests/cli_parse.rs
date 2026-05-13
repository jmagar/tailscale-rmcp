/// Tests for CLI argument parsing (no network, no async).
use rustscale::cli::CliCommand;

#[test]
fn parse_devices() {
    let args = args(&["devices"]);
    let (cmd, json) = CliCommand::parse(&args).expect("should parse");
    assert!(matches!(cmd, CliCommand::Devices));
    assert!(!json);
}

#[test]
fn parse_devices_json_flag() {
    let args = args(&["devices", "--json"]);
    let (cmd, json) = CliCommand::parse(&args).expect("should parse");
    assert!(matches!(cmd, CliCommand::Devices));
    assert!(json);
}

#[test]
fn parse_device_with_id() {
    let args = args(&["device", "abc123"]);
    let (cmd, _json) = CliCommand::parse(&args).expect("should parse");
    assert!(matches!(cmd, CliCommand::Device { id } if id == "abc123"));
}

#[test]
fn parse_routes() {
    let args = args(&["routes", "dev-456"]);
    let (cmd, _) = CliCommand::parse(&args).expect("should parse");
    assert!(matches!(cmd, CliCommand::Routes { device_id } if device_id == "dev-456"));
}

#[test]
fn parse_keys() {
    let args = args(&["keys"]);
    let (cmd, _) = CliCommand::parse(&args).expect("should parse");
    assert!(matches!(cmd, CliCommand::Keys));
}

#[test]
fn parse_acl() {
    let args = args(&["acl"]);
    let (cmd, _) = CliCommand::parse(&args).expect("should parse");
    assert!(matches!(cmd, CliCommand::Acl));
}

#[test]
fn parse_dns() {
    let args = args(&["dns"]);
    let (cmd, _) = CliCommand::parse(&args).expect("should parse");
    assert!(matches!(cmd, CliCommand::Dns));
}

#[test]
fn parse_users() {
    let args = args(&["users"]);
    let (cmd, _) = CliCommand::parse(&args).expect("should parse");
    assert!(matches!(cmd, CliCommand::Users));
}

#[test]
fn parse_authorize() {
    let args = args(&["authorize", "dev-789"]);
    let (cmd, _) = CliCommand::parse(&args).expect("should parse");
    assert!(matches!(cmd, CliCommand::Authorize { device_id } if device_id == "dev-789"));
}

#[test]
fn parse_delete_device_without_confirm() {
    let args = args(&["delete-device", "dev-xxx"]);
    let (cmd, _) = CliCommand::parse(&args).expect("should parse");
    assert!(matches!(
        cmd,
        CliCommand::DeleteDevice {
            device_id,
            confirm: false
        } if device_id == "dev-xxx"
    ));
}

#[test]
fn parse_delete_device_with_confirm() {
    let args = args(&["delete-device", "dev-xxx", "--confirm"]);
    let (cmd, _) = CliCommand::parse(&args).expect("should parse");
    assert!(matches!(
        cmd,
        CliCommand::DeleteDevice {
            device_id,
            confirm: true
        } if device_id == "dev-xxx"
    ));
}

#[test]
#[test]
fn parse_doctor() {
    let args = args(&["doctor"]);
    let (cmd, json) = CliCommand::parse(&args).expect("should parse");
    assert!(matches!(cmd, CliCommand::Doctor));
    assert!(!json);
}

#[test]
fn parse_doctor_with_json_flag() {
    let args = args(&["doctor", "--json"]);
    let (cmd, json) = CliCommand::parse(&args).expect("should parse");
    assert!(matches!(cmd, CliCommand::Doctor));
    assert!(json);
}

#[test]
fn parse_unknown_command_returns_error() {
    let args = args(&["bogus-command"]);
    let result = CliCommand::parse(&args);
    assert!(result.is_err(), "unknown command should fail");
}

#[test]
fn parse_empty_args_returns_error() {
    let args: Vec<String> = vec![];
    let result = CliCommand::parse(&args);
    assert!(result.is_err(), "empty args should fail");
}

#[test]
fn json_flag_works_with_short_form() {
    let args = args(&["keys", "-j"]);
    let (cmd, json) = CliCommand::parse(&args).expect("should parse");
    assert!(matches!(cmd, CliCommand::Keys));
    assert!(json);
}

// ── helper ────────────────────────────────────────────────────────────────────

fn args(strs: &[&str]) -> Vec<String> {
    strs.iter().map(|s| s.to_string()).collect()
}
