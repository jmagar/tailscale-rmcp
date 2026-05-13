# CI/CD Workflows -- syslog-mcp

GitHub Actions configuration for syslog-mcp.

## Workflows

### ci.yml -- Continuous Integration

Runs on every push and pull request.

```yaml
jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - run: cargo clippy -- -D warnings
      - run: cargo fmt -- --check

  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test
```

### docker-publish.yml -- Docker Image Publishing

Triggered on version tag push (`v*`). Builds multi-arch Docker image and pushes to GHCR.

```yaml
on:
  push:
    tags: ['v*']

jobs:
  docker:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}
      - uses: docker/build-push-action@v5
        with:
          push: true
          tags: ghcr.io/jmagar/syslog-mcp:${{ github.ref_name }}
```

### publish-crates.yml -- crates.io Publishing

Triggered on version tag push. Publishes the crate to crates.io.

```yaml
on:
  push:
    tags: ['v*']

jobs:
  publish:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo publish
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
```

### codex-plugin-scanner.yml -- Plugin Validation

Runs on pull requests to validate the Codex plugin manifest.

## Branch strategy

- `main`: Production-ready code
- Feature branches: new features, bug fixes
- PRs required before merge to main
- Version tags (`v0.10.0`) trigger publish workflows

## Secrets

| Secret | Purpose |
| --- | --- |
| `GITHUB_TOKEN` | GHCR Docker image push (automatic) |
| `CARGO_REGISTRY_TOKEN` | crates.io publishing |

## See also

- [PUBLISH.md](PUBLISH.md) -- versioning and release workflow
- [TESTS.md](TESTS.md) -- test configuration
