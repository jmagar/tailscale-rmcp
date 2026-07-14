# Prerequisites

For development:

- Rust 1.86 or newer
- Node.js 18 or newer
- npm
- `mcp-publisher` for registry validation
- `mcporter` for live MCP smoke tests
- Docker if testing container deployment

For runtime:

- Tailscale API key
- tailnet identifier, usually `-`
- HTTP auth when binding outside loopback

Useful checks:

```bash
rustc --version
cargo --version
node --version
npm --version
```
