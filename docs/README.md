# Syslog MCP Documentation

Complete documentation for `syslog-mcp` -- a Rust syslog receiver and MCP server for homelab log intelligence.

## Directory index

### Root-level docs (this directory)

| File | Purpose |
| --- | --- |
| `README.md` | This file -- documentation index |
| `SETUP.md` | Step-by-step setup guide -- clone, build, configure, deploy, verify (at `docs/SETUP.md`) |
| `CONFIG.md` | Configuration reference -- config.toml, env vars, storage budget |
| `CLI.md` | Direct CLI reference -- local search, tail, errors, hosts, correlate, and stats commands |
| `CHECKLIST.md` | Pre-release quality checklist -- version sync, security, CI, registry |
| `GUARDRAILS.md` | Security guardrails -- credentials, Docker, auth, input handling |
| `INVENTORY.md` | Component inventory -- tools, env vars, surfaces, dependencies |

### Subdirectories

| Directory | Scope |
| --- | --- |
| `mcp/` | MCP server docs: auth, transport, tools, resources, testing, deployment |
| `plugin/` | Plugin system docs: manifests, hooks, skills, commands, channels |
| `repo/` | Repository docs: git conventions, scripts, memory, rules |
| `stack/` | Technology stack docs: prerequisites, architecture, Rust dependencies |
| `upstream/` | Upstream service docs (syslog-mcp is self-contained -- no external API) |

### Preserved directories

| Directory | Scope |
| --- | --- |
| `plans/` | Engineering plans and design docs |
| `runbooks/` | Operational runbooks (deploy, maintenance) |
| `sessions/` | Development session notes |
| `superpowers/` | Superpowers plans (storage budget guardrail, etc.) |

## Cross-references

- [CLAUDE.md](../CLAUDE.md) -- project instructions for Claude Code sessions
- [README.md](../README.md) -- user-facing project overview
- [CLI.md](CLI.md) -- direct local CLI command reference
- [SETUP.md](SETUP.md) -- host configuration guide (rsyslog, UniFi, ATT router)
- [CHANGELOG.md](../CHANGELOG.md) -- version history
