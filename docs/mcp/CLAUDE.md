# MCP Documentation

This folder documents the `tailscale-rmcp` MCP surface.

Important files:

- `CONNECT.md` - client connection examples
- `TRANSPORT.md` - stdio and Streamable HTTP behavior
- `TOOLS.md` - action-dispatched `tailscale` tool
- `SCHEMA.md` - input/output schema and protocol metadata
- `RESOURCES.md` - MCP resource behavior
- `AUTH.md` - HTTP auth policy
- `MCPORTER.md` - live smoke testing
- `PUBLISH.md` - registry and package metadata
- `TESTS.md` - validation commands

Do not add policy to MCP shims. `src/mcp/tools.rs` parses arguments only;
business behavior and destructive gates live in `src/app.rs`.
