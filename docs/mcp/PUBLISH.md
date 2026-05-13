# Publishing Strategy -- syslog-mcp

Versioning and release workflow.

## Versioning

Semantic versioning (MAJOR.MINOR.PATCH). Bump type from commit prefix:

| Prefix | Bump | Example |
| --- | --- | --- |
| `feat!:` / `BREAKING CHANGE` | Major | `0.3.1` -> `1.0.0` |
| `feat:` / `feat(scope):` | Minor | `0.3.1` -> `0.4.0` |
| `fix:`, `docs:`, `chore:`, etc. | Patch | `0.3.1` -> `0.3.2` |

## Version sync

All version-bearing files must match. Update together:

| File | Field |
| --- | --- |
| `Cargo.toml` | `version = "X.Y.Z"` in `[package]` |
| `.claude-plugin/plugin.json` | `"version": "X.Y.Z"` |
| `.codex-plugin/plugin.json` | `"version": "X.Y.Z"` |
| `gemini-extension.json` | `"version": "X.Y.Z"` |
| `server.json` | `"version": "X.Y.Z"` |
| `CHANGELOG.md` | New entry under `## X.Y.Z` |

## Publish workflow

```bash
just publish [major|minor|patch]
```

Steps executed:

1. Verify on `main` branch with clean working tree
2. Pull latest from origin
3. Read current version from `Cargo.toml`
4. Compute new version based on bump type
5. Update `Cargo.toml`, plugin manifests, and `gemini-extension.json`
6. Run `cargo check` to update `Cargo.lock`
7. Commit: `release: vX.Y.Z`
8. Tag: `vX.Y.Z`
9. Push to origin with tags (triggers CI/CD publish workflows)

## Package registries

| Registry | Method | Trigger |
| --- | --- | --- |
| crates.io | `cargo publish` via GitHub Actions | `v*` tag push |
| GHCR | Docker image build and push | `v*` tag push |
| MCP Registry | `server.json` under `tv.tootie/syslog-mcp` namespace | manual update |

## server.json

MCP Registry metadata at repo root:

```json
{
  "name": "tv.tootie/syslog-mcp",
  "title": "Syslog MCP",
  "description": "Syslog receiver and MCP server for homelab log intelligence.",
  "version": "0.10.0",
  "packages": [
    {
      "registryType": "oci",
      "identifier": "ghcr.io/jmagar/syslog-mcp:0.10.0",
      "version": "0.10.0"
    }
  ]
}
```

## Verification

After publishing, verify:

```bash
# crates.io
cargo install syslog-mcp --version X.Y.Z

# Docker
docker pull ghcr.io/jmagar/syslog-mcp:vX.Y.Z

# GitHub Release
gh release view vX.Y.Z
```

## See also

- [CICD.md](CICD.md) -- publish workflows triggered by tags
- [DEPLOY.md](DEPLOY.md) -- installation methods
