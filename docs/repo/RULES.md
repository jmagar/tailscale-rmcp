# Coding Rules -- syslog-mcp

Standards and conventions enforced across the repository.

## Git workflow

- `main` branch: production-ready code
- Feature branches for new features and bug fixes
- Pull requests required before merge
- Conventional commit prefixes required

## Commit conventions

```
<type>(<scope>): <description>
```

| Type | Purpose |
| --- | --- |
| `feat` | New feature (minor bump) |
| `feat!` | Breaking change (major bump) |
| `fix` | Bug fix (patch bump) |
| `docs` | Documentation (patch bump) |
| `refactor` | Code refactoring (patch bump) |
| `test` | Test changes (patch bump) |
| `chore` | Maintenance (patch bump) |

Examples:
```
feat(tools): add search filter by facility
fix(db): handle WAL checkpoint timeout
docs(readme): update tool parameter table
refactor(syslog): extract CEF parser to function
```

## Version bumping

Every feature branch push must bump the version in all version-bearing files:
- `Cargo.toml`
- `.claude-plugin/plugin.json`
- `.codex-plugin/plugin.json`
- `gemini-extension.json`
- `server.json`
- `CHANGELOG.md`

All files must have the same version. Never bump only one file.

## Rust code style

- `cargo clippy -- -D warnings` must pass (zero warnings)
- `cargo fmt` must not produce changes
- `set -euo pipefail` in all bash scripts
- Quote all shell variables: `"$var"`
- Use `anyhow::Result` for error handling
- Use `tracing` macros for logging (not `println!`)
- Parameterize all SQL queries (no string interpolation)
- Use `Arc<DbPool>` for shared database access
- Run blocking database operations via `tokio::task::spawn_blocking`

## Never commit

- `.env` files (gitignored)
- Credentials or API keys
- SQLite database files (`data/`)
- Build artifacts (`target/`)
- Temporary/debug files

## Dependency management

- `Cargo.lock` is tracked (binary crate -- reproducible builds)
- Pin major versions in `Cargo.toml` (e.g., `tokio = { version = "1", ... }`)


## See also

- [../mcp/PUBLISH.md](../mcp/PUBLISH.md) -- version bumping and release workflow
- [RECIPES.md](RECIPES.md) -- Justfile recipes for lint, fmt, test
