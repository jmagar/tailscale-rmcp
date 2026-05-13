# Repository Structure -- syslog-mcp

## Directory tree

```
syslog-mcp/
├── .claude-plugin/
│   └── plugin.json              # Claude Code plugin manifest
├── .codex-plugin/
│   └── plugin.json              # Codex plugin manifest
├── .github/
│   └── workflows/
│       ├── ci.yml               # Lint, check, test on push/PR
│       ├── docker-publish.yml   # Build + push Docker image on tag
│       ├── publish-crates.yml   # Publish to crates.io on tag
│       └── codex-plugin-scanner.yml  # Validate Codex manifest on PR
├── docs/
│   ├── mcp/                     # MCP server documentation (20 files)
│   ├── plugin/                  # Plugin surface documentation (11 files)
│   ├── repo/                    # Repository documentation (6 files)
│   ├── stack/                   # Technology stack documentation (4 files)
│   ├── upstream/                # Upstream integration documentation (1 file)
│   ├── plans/                   # Engineering plans
│   ├── runbooks/                # Operational runbooks
│   ├── sessions/                # Development session notes
│   └── superpowers/             # Superpowers plans
├── hooks/
│   ├── hooks.json               # Hook definitions
│   └── scripts/                 # Hook scripts (sync-env, fix-perms, ignore-files)
├── scripts/
│   ├── smoke-test.sh            # Live smoke test
│   ├── backup.sh                # WAL-safe SQLite backup
│   ├── reset-db.sh              # Backup + destructive DB reset




├── skills/
│   └── syslog/
│       └── SKILL.md             # Skill definition
├── src/
│   ├── main.rs                  # Entry point, task wiring, graceful shutdown
│   ├── config.rs                # Config: config.toml + env var overlay
│   ├── db.rs                    # SQLite pool, FTS5, schema, retention, storage budget
│   ├── syslog.rs                # UDP/TCP listeners, parsing, batch writer
│   ├── mcp.rs                   # Facade for MCP modules
│   ├── db/                      # DB queries, models, schema, tests
│   ├── mcp/                     # Axum routes, RMCP adapter, schemas, tools, tests
│   └── syslog/                  # Parser, network listeners, batch writer, tests
├── tests/
│   ├── test_live.sh             # Extended live integration tests
│   ├── mcporter/
│   │   └── test-tools.sh        # mcporter-based tool tests
│   └── TEST_COVERAGE.md         # Test coverage documentation
├── config/
│   └── mcporter.json            # mcporter client config
├── data/                        # SQLite database (gitignored)
│
├── .env.example                 # Environment variable template
├── AGENTS.md                    # Agent declarations (none)
├── CHANGELOG.md                 # Version history
├── CLAUDE.md                    # Claude Code project instructions
├── Cargo.toml                   # Rust package manifest
├── Cargo.lock                   # Dependency lock file (tracked)
├── config.toml                  # Local dev config (not in Docker)
├── docker-compose.yml           # Container orchestration
├── Dockerfile                   # Multi-stage container build
├── entrypoint.sh                # Container entrypoint
├── gemini-extension.json        # Gemini CLI manifest
├── Justfile                     # Task runner recipes
├── LICENSE                      # MIT license
├── README.md                    # User-facing documentation
├── server.json                  # MCP Registry entry
```

## Source code

| File | Purpose |
| --- | --- |
| `src/main.rs` | Entry point: wires config, DB, syslog, MCP; starts background tasks; graceful shutdown |
| `src/config.rs` | Three-layer config: defaults + config.toml + env vars; validation |
| `src/db.rs` + `src/db/` | SQLite connection pool, schema init, migrations, FTS5, all query functions, storage budget enforcement |
| `src/syslog.rs` + `src/syslog/` | UDP/TCP listeners, RFC 3164/5424 parsing, UniFi CEF extraction, mpsc batch writer |
| `src/mcp.rs` + `src/mcp/` | MCP facade plus Axum routes, bearer auth middleware, RMCP Streamable HTTP adapter, schemas, tools, and tests |

## Plugin surfaces

| Directory | Surface |
| --- | --- |
| `skills/` | Skill definition (SKILL.md) |
| `hooks/` | Lifecycle hooks (sync-env, fix-perms, ignore-files) |

No agents, commands, channels, output styles, or schedules.

## See also

- [RULES.md](RULES.md) -- coding conventions
- [RECIPES.md](RECIPES.md) -- Justfile recipes
- [SCRIPTS.md](SCRIPTS.md) -- scripts reference
