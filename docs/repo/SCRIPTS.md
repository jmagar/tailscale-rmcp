# Scripts

| Script | Purpose |
|---|---|
| `scripts/install.sh` | Download and install `rtailscale` from GitHub Releases. |
| `scripts/bump-version.sh` | Update version surfaces. |
| `scripts/check-version-sync.sh` | Verify package/version sync. |
| `scripts/check-runtime-current.sh` | Check whether runtime binary matches the repo. |
| `scripts/refresh-docs.sh` | Refresh upstream reference snapshots. |
| `scripts/sync-cargo.sh` | Synchronize package metadata from Cargo. |
| `scripts/validate-plugin-layout.sh` | Validate plugin file layout. |
| `scripts/block-env-commits.sh` | Guard against committing secret env files. |

Package scripts live under `packages/tailscale-rmcp/scripts/`.
