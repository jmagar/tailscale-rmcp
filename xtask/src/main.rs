//! cargo xtask — repo automation for tailscale-rmcp
//!
//! Usage: cargo xtask <command>
//!
//! Commands:
//!   dist         Build release binary, copy to bin/, update Git LFS
//!   ci           Run all checks (fmt, clippy, test, audit)
//!   symlink-docs Symlink CLAUDE.md → AGENTS.md + GEMINI.md everywhere
//!   check-env    Validate required env vars are set

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("dist") => dist(),
        Some("ci") => ci(),
        Some("symlink-docs") => symlink_docs(),
        Some("check-env") => check_env(),
        Some(other) => {
            bail!("unknown xtask command: {other}\n\nAvailable: dist, ci, symlink-docs, check-env")
        }
        None => {
            bail!("usage: cargo xtask <command>\n\nAvailable: dist, ci, symlink-docs, check-env")
        }
    }
}

/// Build the release binary and copy it to bin/tailscale (Git LFS-tracked).
fn dist() -> Result<()> {
    let root = workspace_root();
    println!("xtask dist: building release binary...");
    run(Command::new("cargo")
        .args(["build", "--release", "--locked"])
        .current_dir(&root))
    .context("cargo build --release failed")?;

    // Determine target dir (respects CARGO_TARGET_DIR)
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| root.join("target"));
    let src = target_dir.join("release").join("tailscale");
    let bin_dir = root.join("bin");
    std::fs::create_dir_all(&bin_dir).context("creating bin/")?;
    let dst = bin_dir.join("tailscale");
    std::fs::copy(&src, &dst).with_context(|| format!("copying {src:?} → {dst:?}"))?;

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dst)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dst, perms)?;
    }

    println!("xtask dist: binary written to {}", dst.display());
    println!("xtask dist: run `git add bin/tailscale && git commit` to update Git LFS");
    Ok(())
}

/// Run all CI checks: fmt, clippy, nextest, taplo, audit.
fn ci() -> Result<()> {
    let root = workspace_root();
    println!("xtask ci: running all checks...");

    run(Command::new("cargo")
        .args(["fmt", "--", "--check"])
        .current_dir(&root))
    .context("cargo fmt check failed")?;

    run(Command::new("cargo")
        .args(["clippy", "--", "-D", "warnings"])
        .current_dir(&root))
    .context("cargo clippy failed")?;

    // Use nextest if available, otherwise fall back to cargo test
    let nextest = Command::new("cargo")
        .args(["nextest", "run", "--profile", "ci"])
        .current_dir(&root)
        .status();
    match nextest {
        Ok(s) if s.success() => {}
        Ok(_) => bail!("cargo nextest run --profile ci failed"),
        Err(_) => {
            // nextest not installed — fall back
            eprintln!("xtask ci: cargo-nextest not found, falling back to cargo test");
            run(Command::new("cargo").args(["test"]).current_dir(&root))
                .context("cargo test failed")?;
        }
    }

    // taplo if available
    let taplo = Command::new("taplo")
        .args(["check"])
        .current_dir(&root)
        .status();
    match taplo {
        Ok(s) if s.success() => {}
        Ok(_) => bail!("taplo check failed"),
        Err(_) => eprintln!("xtask ci: taplo not found, skipping TOML format check"),
    }

    // cargo audit if available
    let audit = Command::new("cargo")
        .args(["audit"])
        .current_dir(&root)
        .status();
    match audit {
        Ok(s) if s.success() => {}
        Ok(_) => bail!("cargo audit failed"),
        Err(_) => eprintln!("xtask ci: cargo-audit not found, skipping audit"),
    }

    println!("xtask ci: all checks passed");
    Ok(())
}

/// Symlink CLAUDE.md → AGENTS.md and GEMINI.md everywhere in the repo.
fn symlink_docs() -> Result<()> {
    let root = workspace_root();
    println!("xtask symlink-docs: creating AGENTS.md + GEMINI.md symlinks...");

    let mut count = 0;
    visit_claude_md(&root, &root, &mut count)?;
    println!("xtask symlink-docs: processed {count} CLAUDE.md file(s)");
    Ok(())
}

fn visit_claude_md(dir: &Path, root: &Path, count: &mut usize) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Skip .git and target
        if name_str == ".git" || name_str == "target" {
            continue;
        }

        if path.is_dir() {
            visit_claude_md(&path, root, count)?;
        } else if name_str == "CLAUDE.md" {
            let dir_path = path.parent().unwrap();
            for link_name in &["AGENTS.md", "GEMINI.md"] {
                let link_path = dir_path.join(link_name);
                // Remove existing symlink/file first
                if link_path.exists() || link_path.symlink_metadata().is_ok() {
                    std::fs::remove_file(&link_path)?;
                }
                #[cfg(unix)]
                std::os::unix::fs::symlink("CLAUDE.md", &link_path)
                    .with_context(|| format!("symlink CLAUDE.md → {link_path:?}"))?;
                #[cfg(not(unix))]
                eprintln!(
                    "xtask symlink-docs: skipping symlink on non-Unix ({})",
                    link_path.display()
                );
            }
            *count += 1;
            println!("  {} → AGENTS.md, GEMINI.md", path.display());
        }
    }
    Ok(())
}

/// Validate that required env vars are set.
fn check_env() -> Result<()> {
    println!("xtask check-env: validating required environment variables...");

    let required = [
        (
            "TAILSCALE_API_KEY",
            "Tailscale API key — create at https://login.tailscale.com/admin/settings/keys",
        ),
        (
            "TAILSCALE_TAILNET",
            "Tailnet identifier — org domain (e.g. example.com) or '-' for personal account",
        ),
    ];

    let mut missing = Vec::new();
    for (var, hint) in &required {
        match std::env::var(var) {
            Ok(v) if !v.is_empty() => println!("  OK  {var}"),
            _ => {
                eprintln!("  MISSING  {var}  ({hint})");
                missing.push(*var);
            }
        }
    }

    if !missing.is_empty() {
        bail!(
            "missing required env vars: {}\n\nCopy .env.example → .env and fill in the values.",
            missing.join(", ")
        );
    }

    println!("xtask check-env: all required env vars set");
    Ok(())
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is xtask/, so go up one level
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent().unwrap().to_path_buf()
}

fn run(cmd: &mut Command) -> Result<()> {
    let status = cmd.status().with_context(|| format!("running {cmd:?}"))?;
    if !status.success() {
        bail!("command failed with exit code: {:?}", status.code());
    }
    Ok(())
}
