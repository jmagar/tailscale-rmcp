# rmcp 1.6 axum-extension propagation spike (syslog-mcp-brt0.10)

## Outcome

**Pattern (a) — direct extension propagation — WORKS.** Pattern (b) and Pattern (c) are NOT needed.

Confirmed empirically against `rmcp 1.6.0` with feature `transport-streamable-http-server`, in both `stateful_mode(true)` and `stateful_mode(false)`. Spike test: `tests/spike_rmcp_extensions.rs`.

The earlier `framework-docs-researcher` finding ("rmcp's `RequestContext` carries only MCP protocol metadata, not the underlying HTTP request") was incorrect for rmcp 1.6.0 — that may have been true for an earlier version, but the 1.6 line publishes `http::request::Parts` into the JSON-RPC request's extensions before dispatching to the tool handler.

## How it works in rmcp 1.6.0

1. Axum middleware runs first (above the `nest_service("/mcp", StreamableHttpService)` call) and inserts arbitrary values into the request: `request.extensions_mut().insert(AuthContext { ... })`.
2. `StreamableHttpService::handle()` decomposes the request into `Parts` + body, parses the JSON-RPC envelope, and then **calls `req.request.extensions_mut().insert(part)`** before dispatching — see `rmcp-1.6.0/src/transport/streamable_http_server/tower.rs:1039` (stateful path) and `tower.rs:1179` (stateless path). The `part` is the full `http::request::Parts`, which carries every axum extension set upstream.
3. `service::server::serve_inner` then constructs the `RequestContext` with `extensions: request.extensions().clone()` (`rmcp-1.6.0/src/service/server.rs:210` and `service.rs:955`), so `RequestContext.extensions` contains `http::request::Parts`, whose own `extensions` field carries the original axum-layer values.

The official rustdoc on `StreamableHttpService` documents the pattern verbatim: `rmcp-1.6.0/src/transport/streamable_http_server/tower.rs:473-495`.

## Reading AuthContext inside a tool handler

```rust
async fn call_tool(
    &self,
    request: CallToolRequestParams,
    context: RequestContext<RoleServer>,
) -> Result<CallToolResult, ErrorData> {
    let parts = context
        .extensions
        .get::<axum::http::request::Parts>()
        .ok_or_else(|| ErrorData::internal_error("missing http parts", None))?;

    let auth = parts
        .extensions
        .get::<AuthContext>()
        .cloned()
        .ok_or_else(|| {
            // Fail-closed per epic Locked Decisions.
            ErrorData::invalid_request("forbidden: missing auth context", None)
        })?;

    // scope check, then dispatch...
}
```

Setting it from middleware:

```rust
async fn auth_middleware(mut req: Request, next: Next) -> Response {
    let ctx = AuthContext { subject: "...".into(), scopes: vec![...] };
    req.extensions_mut().insert(ctx);
    next.run(req).await
}

// Router wiring:
Router::new()
    .nest_service("/mcp", streamable_http_service(...))
    .layer(middleware::from_fn(auth_middleware))
```

## stateful_mode requirement

**No flip required.** Pattern (a) works under syslog-mcp's current `stateful_mode(false)` setup. Both branches in `tower.rs` inject the parts into the JSON-RPC request extensions; the only difference is that stateful mode also routes the message through `SessionManager`, which is orthogonal to extension propagation.

This means:
- syslog-mcp keeps `with_stateful_mode(false)` (current `src/mcp/rmcp_server.rs:148`).
- `LocalSessionManager` continues to be used as-is.
- No `Arc<DashMap<SessionId, AuthContext>>` is required on `AppState` — auth lives on each request.
- No `tokio::task_local!` is required — no scoping concerns across rmcp-spawned tasks.

## Patterns evaluated

### Pattern (a) — direct extension propagation [CHOSEN]
- **Verdict**: works against rmcp 1.6.0, no transport-mode change required.
- **Pros**: zero AppState surface area for auth; fail-closed is trivial; per-request lifetime matches the auth check perfectly; no race conditions.
- **Cons**: ties tool handlers to `axum::http::request::Parts` import — minor coupling, acceptable for syslog-mcp's single-transport reality.

### Pattern (b) — AppState session-keyed map [REJECTED]
- Would require flipping `stateful_mode` to `true` to obtain stable session IDs, plus an `Arc<DashMap<SessionId, AuthContext>>` on `AppState`, plus middleware that fishes the `Mcp-Session-Id` header out of the request. Three moving parts where one suffices.
- Race window: middleware writes to the map; handler reads it; eviction must happen on session terminate. Avoidable bugs.
- Also: the lab repo uses a variant of pattern (b), but lab predates the rmcp-1.6 docs that explicitly support pattern (a). Lab can simplify later.

### Pattern (c) — tokio task-local around `StreamableHttpService::call` [REJECTED]
- Requires wrapping `StreamableHttpService` in a tower service that scopes a `task_local!` around the `call`. Works in principle, but rmcp internally calls `tokio::spawn` for stateful sessions (see `tower.rs` `spawn_session_worker`), and task-locals do NOT propagate across `tokio::spawn`. So this is unsound under stateful_mode without explicit `LocalKey::scope` wrapping every spawn — out of our reach inside rmcp.
- Even under stateless mode (where the dispatch is in-place), pattern (a) has equal ergonomics with no `task_local!` boilerplate.

## Gotchas

1. **Type-name ambiguity**: `axum::http::request::Parts` and `http::request::Parts` are the same type; axum re-exports the `http` crate. The handler import must match what middleware inserted (in our spike, both used `axum::http`). If a future middleware pulls from `hyper::http::request::Parts` and the handler reads `axum::http::request::Parts`, they're still the same `TypeId` because axum and hyper depend on the same `http` crate version, but pin the crate explicitly to avoid surprises if `http` ever splits.
2. **Body is consumed before tool dispatch**: only `Parts` reaches the handler — no streaming body access. Auth middleware MUST do its work BEFORE rmcp consumes the body, which is the natural axum middleware ordering.
3. **`#[non_exhaustive]` on `RequestContext`**: cannot pattern-match exhaustively; always construct/access by field name. Already true in syslog-mcp's existing handlers.
4. **Cloning cost**: `RequestContext.extensions` is cloned in `service/server.rs:210`. `http::request::Parts` is `Clone`, but the Parts contains an `Extensions` map which is internally a `HashMap<TypeId, Box<dyn Any>>`. Cloning copies the map but the `Box<dyn Any>` inside is NOT cloned — clones share. This is fine for a read-only AuthContext, but DO NOT mutate inserted values.
5. **Spike test isolation**: the spike test uses `tower::ServiceExt::oneshot` so each test gets a fresh router. Production wiring does NOT need anything special.

## References

- Spike test: `tests/spike_rmcp_extensions.rs` (DELETE AFTER syslog-mcp-brt0.10 closed)
- Documented pattern: `~/.cargo/registry/src/index.crates.io-*/rmcp-1.6.0/src/transport/streamable_http_server/tower.rs:473-495`
- Injection sites: `tower.rs:1039` (stateful), `tower.rs:1102` (stateful init), `tower.rs:1179` (stateless)
- RequestContext construction: `service/server.rs:210` and `service.rs:955`
- Current syslog-mcp setup: `src/mcp/rmcp_server.rs:146-163`
