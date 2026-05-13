# Pre-commit Hook Configuration -- syslog-mcp

Hooks run as Claude Code lifecycle hooks via `hooks/hooks.json`.

## Hook configuration

Hooks are defined in `hooks/hooks.json` and enforced by Claude Code during sessions:

| Hook | Script | Purpose |
| --- | --- | --- |
| `sync-env` | `hooks/scripts/sync-env.sh` | Ensures `.env.example` documents all variables read by the server |
| `fix-env-perms` | `hooks/scripts/fix-env-perms.sh` | Sets `.env` to `chmod 600` if present |


## Manual checks

Run checks manually outside of Claude Code:

```bash
# Plugin manifest validation
just check-contract

# Docker security check


# No baked env vars in Docker image


# Outdated dependencies


# Ignore file patterns

```

## Rust-specific checks

Before committing, run:

```bash
just lint        # cargo clippy -- -D warnings
just fmt         # cargo fmt
just test        # cargo test
```

These are not automated as git hooks but are enforced in CI.

## See also

- [CICD.md](CICD.md) -- CI workflow enforces lint and test
- [../GUARDRAILS.md](../GUARDRAILS.md) -- security patterns enforced by hooks
