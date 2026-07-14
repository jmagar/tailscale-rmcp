# Setup Guide

## 1. Get a Tailscale API key

Create an API key in the Tailscale admin console:

```text
https://login.tailscale.com/admin/settings/keys
```

Use the minimum capability needed for the actions exposed to clients.

## 2. Configure credentials

Host installs load `~/.tailscale-mcp/.env`; containers load `/data/.env`.
Process environment values win over files.

```bash
mkdir -p ~/.tailscale-mcp
chmod 700 ~/.tailscale-mcp
cat > ~/.tailscale-mcp/.env <<'EOF'
TAILSCALE_API_KEY=tskey-api-...
TAILSCALE_TAILNET=-
EOF
chmod 600 ~/.tailscale-mcp/.env
```

`TAILSCALE_TAILNET=-` targets the default personal tailnet. Organization
tailnets usually use the org domain.

## 3. Run the CLI

```bash
npx -y tailscale-rmcp devices --json
npx -y tailscale-rmcp users --json
```

The npm package launches the Rust binary `rtailscale`.

## 4. Run stdio MCP

Use stdio for local child-process MCP clients:

```bash
npx -y tailscale-rmcp mcp
```

Claude Code example:

```json
{
  "mcpServers": {
    "tailscale": {
      "command": "npx",
      "args": ["-y", "tailscale-rmcp", "mcp"],
      "env": {
        "TAILSCALE_API_KEY": "tskey-api-...",
        "TAILSCALE_TAILNET": "-"
      }
    }
  }
}
```

## 5. Run HTTP MCP

Loopback development:

```bash
TAILSCALE_MCP_HOST=127.0.0.1 npx -y tailscale-rmcp serve
curl -sf http://127.0.0.1:40040/health
```

Shared deployments must configure one of:

- `TAILSCALE_MCP_TOKEN`
- `TAILSCALE_MCP_AUTH_MODE=oauth`
- `TAILSCALE_NOAUTH=true` when an upstream gateway already enforces auth

## 6. Docker

```bash
cp .env.example .env
docker compose up -d
docker compose logs -f
```

The container exposes MCP on port `40040` by default and uses `/data/.env`.

## 7. Plugin setup

From this checkout:

```bash
claude plugin install plugins/tailscale
```

The plugin hook calls:

```bash
${CLAUDE_PLUGIN_ROOT}/bin/rtailscale setup plugin-hook
```

## Troubleshooting

| Symptom | Check |
|---|---|
| Missing API key | Set `TAILSCALE_API_KEY` in env or `~/.tailscale-mcp/.env`. |
| HTTP startup refused | Non-loopback binds need bearer auth, OAuth, or `TAILSCALE_NOAUTH=true`. |
| Stdio client logs JSON errors | Ensure the client command includes `mcp`. |
| Delete rejected | Set `TAILSCALE_ALLOW_DESTRUCTIVE=true` and pass `confirm=true`. |
