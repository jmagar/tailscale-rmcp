# syntax=docker/dockerfile:1.7

FROM rust:1.90-slim-bookworm AS builder

WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Cache dependencies — build a dummy binary first so dep compilation is cached
COPY Cargo.toml Cargo.lock ./
# Strip xtask workspace member (not in Docker build context)
RUN sed -i 's/, "xtask"//g; s/"xtask", //g; s/"xtask"//g' Cargo.toml
RUN --mount=type=cache,id=tailscale-rmcp-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=tailscale-rmcp-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=tailscale-rmcp-cargo-target,target=/app/target,sharing=locked \
    mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release --locked -p tailscale-rmcp && rm -rf src

# Build the real binary
COPY src/ src/
RUN --mount=type=cache,id=tailscale-rmcp-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=tailscale-rmcp-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=tailscale-rmcp-cargo-target,target=/app/target,sharing=locked \
    touch src/main.rs && cargo build --release --locked -p tailscale-rmcp && \
    cp target/release/rtailscale /usr/local/bin/tailscale-mcp

# Runtime image
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates curl gosu && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/bin/tailscale-mcp /usr/local/bin/tailscale-mcp

# Create unprivileged service user and data directory
RUN groupadd --gid 1000 tailscale-rmcp && \
    useradd --uid 1000 --gid tailscale-rmcp --no-create-home --shell /sbin/nologin tailscale-rmcp && \
    mkdir -p /data && \
    chown tailscale-rmcp:tailscale-rmcp /data

ENV RUST_LOG=info


EXPOSE 40040/tcp

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -sf http://localhost:40040/health || exit 1

LABEL io.modelcontextprotocol.server.name="tv.tootie/tailscale-rmcp"

# Note: the binary inside the container is named tailscale-mcp to avoid
# conflicting with any Tailscale CLI tooling that might be mounted in.
# The host-side binary (in bin/) is named tailscale for consistency with
# Cargo.toml's [[bin]] name.
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh
ENTRYPOINT ["/entrypoint.sh"]
CMD ["serve", "mcp"]
