# Guardrails

## Credential boundaries

- `TAILSCALE_API_KEY` is loaded from config or env.
- MCP tool arguments never accept Tailscale credentials.
- OAuth and bearer secrets are HTTP transport credentials, not tool inputs.
- Do not log full API keys or bearer tokens.

## Destructive actions

`delete_device` requires two independent signals:

1. Server opt-in with `TAILSCALE_ALLOW_DESTRUCTIVE=true`.
2. Caller confirmation with `confirm=true` or CLI `--confirm`.

This gate lives in `src/app.rs::TailscaleService::delete_device`.

## HTTP auth

HTTP MCP refuses non-loopback startup unless one of these is configured:

- `TAILSCALE_MCP_TOKEN`
- `TAILSCALE_MCP_AUTH_MODE=oauth`
- `TAILSCALE_NOAUTH=true` behind an authenticated gateway

Stdio MCP is a trusted local process boundary and does not use HTTP auth.

## Result contract

Every successful MCP tool call returns:

```json
{
  "ok": true,
  "data": {},
  "error": null
}
```

Tool errors return:

```json
{
  "ok": false,
  "data": null,
  "error": {
    "code": "not_found",
    "message": "Device not found",
    "status": 404,
    "action": "device",
    "upstream": "tailscale"
  }
}
```

Tailscale API errors preserve status, stable code, message, hint when known, and
diagnostic upstream body when available.

## MCP metadata

The server advertises:

- `serverInfo.name = tailscale-rmcp`
- `serverInfo.title = Tailscale RMCP`
- server icons
- one tool named `tailscale`
- tool `outputSchema`
- tool `execution.taskSupport = forbidden`
- tool, resource, and prompt `_meta` under `ai.dinglebear/tailscale-rmcp`
- resource and prompt decorative icons

Metadata is descriptive. Authorization and destructive gates remain enforced in
the service layer.
