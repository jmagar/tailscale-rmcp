# MCP Server Documentation -- syslog-mcp

Index for the `mcp/` documentation subdirectory. These docs cover the MCP server implementation, transport, tools, testing, and deployment.

## File index

| File | Purpose |
| --- | --- |
| `AUTH.md` | Authentication: bearer token, constant-time comparison, unauthenticated endpoints |
| `CICD.md` | CI/CD workflows: lint, test, Docker publish, crates.io publish |
| `CONNECT.md` | Client connection methods: plugin install, Claude Code, Codex, Gemini, curl |
| `DEPLOY.md` | Deployment: local dev, Docker, Docker Compose, port assignment |
| `DEV.md` | Development workflow: build cycle, adding tools, debugging, code style |
| `ELICITATION.md` | MCP elicitation: syslog-mcp does not use elicitation (read-only tools) |
| `ENV.md` | Environment variable reference (concise cross-ref to CONFIG.md) |
| `LOGS.md` | Logging: RUST_LOG, tracing, structured output, error handling |
| `MCPORTER.md` | Live smoke testing with mcporter |
| `MCPUI.md` | MCP UI patterns: schema annotations for client rendering |
| `PATTERNS.md` | Code patterns: flat tool dispatch, run_db helper, batch writer |
| `PRE-COMMIT.md` | Pre-commit hook configuration |
| `PUBLISH.md` | Publishing: versioning, crates.io, GHCR, MCP registry |
| `RESOURCES.md` | MCP resources: schema resource and URI conventions |
| `SCHEMA.md` | Tool schema documentation: JSON Schema definitions in Rust |
| `TESTS.md` | Testing: cargo test, live smoke tests, test coverage |
| `TOOLS.md` | MCP tools reference: one `syslog` tool with all 8 action shapes |
| `TRANSPORT.md` | RMCP Streamable HTTP transport, stateless mode, port assignment |
| `WEBMCP.md` | Web MCP: CORS configuration, browser access restrictions |
