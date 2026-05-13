# MCP Elicitation -- syslog-mcp

## Overview

Elicitation is an MCP protocol capability that allows servers to request information from users interactively. syslog-mcp does not use elicitation.

## Why no elicitation

syslog-mcp is a self-contained syslog receiver with no external API dependencies. There are no upstream credentials to collect at first-run. All configuration is handled via environment variables and `config.toml`.

Additionally, all 8 MCP actions are read-only. There are no destructive operations that would benefit from confirmation gates via elicitation.

## Configuration entry points

Instead of elicitation, syslog-mcp uses:

| Method | Purpose |
| --- | --- |
| Environment variables | All runtime configuration |
| `config.toml` | Local development overrides |
| Plugin `userConfig` | MCP URL and API token when installed via Claude Code plugin |

## See also

- [ENV.md](ENV.md) -- environment variable reference
- [AUTH.md](AUTH.md) -- bearer token configuration
