# Auth Routing Note

`tailscale-rmcp` uses `lab-auth` for HTTP auth. The RMCP Streamable HTTP service
is mounted under `/mcp`; the auth layer is applied to that subtree before RMCP
handles JSON-RPC requests.

Policy selection happens in `main.rs::resolve_auth_policy`:

- loopback or explicit `no_auth` becomes `AuthPolicy::LoopbackDev`
- OAuth mode builds a `lab_auth::state::AuthState`
- bearer mode mounts static-token auth without OAuth state

OAuth settings can come from `config.toml` or environment. Environment wins so
container and gateway deployments can override image defaults.
