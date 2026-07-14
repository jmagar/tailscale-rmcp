# Rust Development

Minimum supported Rust version: 1.86.

```bash
cargo fmt --check
cargo test --locked
cargo clippy -- -D warnings
cargo build --release
```

Release binary:

```text
target/release/rtailscale
```

The crate uses RMCP 1.6 with Streamable HTTP server and stdio transport
features enabled. The source layout follows the RMCP family rule that `cli.rs`
and `mcp/tools.rs` are thin shims over `TailscaleService`.

For musl builds on this host, clear the global sccache wrapper if it cannot find
the target standard library:

```bash
RUSTC_WRAPPER= cargo build --release --target x86_64-unknown-linux-musl
```
