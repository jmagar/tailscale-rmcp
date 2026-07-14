# Technology Choices

| Area | Choice | Reason |
|---|---|---|
| Language | Rust | Small static binary, async HTTP, reliable packaging. |
| MCP SDK | `rmcp` | Native Streamable HTTP and stdio support. |
| HTTP server | Axum | Simple router/layer integration with RMCP and auth. |
| Auth | `lab-auth` | Shared RMCP-family bearer/OAuth implementation. |
| HTTP client | `reqwest` with rustls | Tailscale REST API calls without OpenSSL dependency. |
| Config | `toml`, env vars, dotenvy | Works for host, Docker, and plugin installs. |
| Package | npm launcher | Easy stdio MCP installation with downloaded Rust binary. |
| Registry | `server.json` | MCP Registry package and remote metadata. |

There is no database, ingest loop, web UI, or local Tailscale daemon control in
this project.
