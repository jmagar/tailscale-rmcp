# syntax=docker/dockerfile:1.7

FROM rust:1.90-slim-bookworm AS builder

WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Cache dependencies — build a dummy binary first so dep compilation is cached
COPY Cargo.toml Cargo.lock ./
RUN --mount=type=cache,id=rustscale-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=rustscale-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=rustscale-cargo-target,target=/app/target,sharing=locked \
    mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release --locked && rm -rf src

# Build the real binary
COPY src/ src/
RUN --mount=type=cache,id=rustscale-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=rustscale-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=rustscale-cargo-target,target=/app/target,sharing=locked \
    touch src/main.rs && cargo build --release --locked && \
    cp target/release/tailscale /usr/local/bin/tailscale-mcp

# Runtime image
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/bin/tailscale-mcp /usr/local/bin/tailscale-mcp

# Create unprivileged service user and data directory
RUN groupadd --gid 1000 rustscale && \
    useradd --uid 1000 --gid rustscale --no-create-home --shell /sbin/nologin rustscale && \
    mkdir -p /data && \
    chown rustscale:rustscale /data

ENV RUST_LOG=info

USER 1000:1000

EXPOSE 40040/tcp

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -sf http://localhost:40040/health || exit 1

LABEL io.modelcontextprotocol.server.name="tv.tootie/rustscale"

# Note: the binary inside the container is named tailscale-mcp to avoid
# conflicting with any Tailscale CLI tooling that might be mounted in.
# The host-side binary (in bin/) is named tailscale for consistency with
# Cargo.toml's [[bin]] name.
CMD ["tailscale-mcp", "serve", "mcp"]
