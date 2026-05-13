# Prerequisites -- syslog-mcp

Required tools and versions before developing or deploying.

## Development

| Tool | Version | Purpose | Install |
| --- | --- | --- | --- |
| Rust | 1.86+ | Compiler toolchain | `rustup default stable` |
| cargo | (bundled) | Build system, package manager | (included with Rust) |
| just | latest | Task runner | `cargo install just` |
| Docker | 24+ | Container deployment | [docs.docker.com](https://docs.docker.com/get-docker/) |
| Docker Compose | v2+ | Orchestration | (included with Docker Desktop) |
| curl | any | Health checks, API testing | System package |
| jq | any | JSON formatting (optional) | System package |
| openssl | any | Token generation | System package |

## Runtime (Docker)

The Docker image includes all runtime dependencies. No additional tools needed on the host beyond Docker.

## Runtime (bare metal)

| Dependency | Purpose |
| --- | --- |
| glibc | Standard C library (Debian bookworm-slim compatible) |
| ca-certificates | TLS certificate bundle |
| curl | Container health check |

SQLite is statically linked via `rusqlite` with the `bundled` feature -- no system SQLite required.

## Optional tools

| Tool | Purpose |
| --- | --- |
| mcporter | MCP client for smoke testing |
| rsyslog | Syslog forwarder for sending test messages |
| logger | CLI tool for sending test syslog messages |
| sqlite3 | Direct database inspection |

## System requirements

| Resource | Minimum | Recommended |
| --- | --- | --- |
| RAM | 64 MB | 256 MB |
| Disk | 100 MB + DB size | 2 GB |
| CPU | 1 core | 2 cores |
| Network | UDP/TCP port 1514, TCP port 3100 | Same |

## See also

- [SETUP.md](../SETUP.md) -- full setup guide
- [TECH.md](TECH.md) -- technology choices
