# Plugin Checklist -- syslog-mcp

Pre-release and quality checklist. Complete all items before tagging a release.

## Version and metadata

- [ ] All version-bearing files in sync: `Cargo.toml`, `.claude-plugin/plugin.json`, `.codex-plugin/plugin.json`, `gemini-extension.json`, `server.json`
- [ ] `CHANGELOG.md` has an entry for the new version
- [ ] README version badge is correct

## Configuration

- [ ] `.env.example` documents every environment variable the server reads
- [ ] `.env.example` has no actual secrets -- only placeholders
- [ ] `.env` is in `.gitignore` and `.dockerignore`

## Documentation

- [ ] `CLAUDE.md` is current and matches repo structure
- [ ] `README.md` has up-to-date tool reference and environment variable table
- [ ] `skills/syslog/SKILL.md` has correct frontmatter and tool descriptions
- [ ] Setup instructions work from a clean clone

## Security

- [ ] No credentials in code, docs, or git history
- [ ] `.gitignore` includes `.env`, `*.secret`, credentials files
- [ ] `.dockerignore` includes `.env`, `.git/`, `*.secret`

- [ ] `/health` endpoint is unauthenticated; `/mcp` requires bearer auth when `SYSLOG_MCP_TOKEN` is set
- [ ] Container runs as non-root (UID 1000)
- [ ] No baked environment variables in Docker image
- [ ] Bearer token comparison uses constant-time equality (`subtle::ConstantTimeEq`)

## Build and test

- [ ] Docker image builds: `just docker-build`
- [ ] Docker healthcheck passes: `just health`
- [ ] CI pipeline passes: `just lint && just test`
- [ ] Live smoke test passes: `just test-live`
- [ ] `cargo clippy -- -D warnings` produces zero warnings

## Deployment

- [ ] `docker-compose.yml` uses correct ports (1514 UDP/TCP, 3100 TCP)
- [ ] `entrypoint.sh` is executable
- [ ] SWAG reverse proxy config tested (see `docs/syslog.subdomain.conf`)

## Registry (if publishing)

- [ ] `server.json` for MCP registry is valid JSON with correct version
- [ ] OCI image published to `ghcr.io/jmagar/syslog-mcp`
- [ ] Crate published to crates.io (if applicable)
- [ ] DNS verification for `tv.tootie/syslog-mcp`

## Marketplace (if applicable)

- [ ] Entry in `claude-homelab` marketplace manifest
- [ ] Plugin installs correctly: `/plugin marketplace add jmagar/claude-homelab`
