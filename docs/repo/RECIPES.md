# Justfile Recipes -- syslog-mcp

Run `just --list` to see all available recipes.

## Development

| Recipe | Command | Description |
| --- | --- | --- |
| `just dev` | `cargo run` | Start dev server |
| `just build` | `cargo build` | Debug build |
| `just release` | `cargo build --release` | Release build |
| `just check` | `cargo check` | Type check without building |
| `just lint` | `cargo clippy -- -D warnings` | Lint with zero-warning policy |
| `just fmt` | `cargo fmt` | Auto-format code |
| `just test` | `cargo test` | Run test suite |
| `just clean` | `cargo clean` | Remove build artifacts |

## Docker

| Recipe | Command | Description |
| --- | --- | --- |
| `just docker-build` | `docker build -t syslog-mcp .` | Build Docker image |
| `just up` | `docker compose up -d` | Start containers |
| `just down` | `docker compose down` | Stop containers |
| `just restart` | `docker compose restart` | Restart containers |
| `just logs` | `docker compose logs -f` | Tail container logs |

## Testing

| Recipe | Command | Description |
| --- | --- | --- |
| `just health` | `curl -sf http://localhost:3100/health \| jq .` | Health check |
| `just test-live` | `bash tests/test_live.sh` | Run live smoke tests |

## Setup and security

| Recipe | Command | Description |
| --- | --- | --- |
| `just setup` | `cp -n .env.example .env` | Initialize .env file |
| `just gen-token` | `openssl rand -hex 32` | Generate bearer token |
| `just validate-skills` | Check SKILL.md exists | Verify skill files |

## Publishing

| Recipe | Command | Description |
| --- | --- | --- |
| `just publish [bump]` | Bump, tag, push | Release with major/minor/patch bump |

The `publish` recipe:
1. Verifies clean `main` branch
2. Bumps version in Cargo.toml and all plugin manifests
3. Commits as `release: vX.Y.Z`
4. Tags `vX.Y.Z`
5. Pushes to origin (triggers CI/CD publish workflows)
