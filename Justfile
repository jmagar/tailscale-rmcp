dev:
    cargo run -- serve mcp

build:
    cargo build

release:
    cargo build --release

check:
    cargo check

lint:
    cargo clippy -- -D warnings

fmt:
    cargo fmt

test:
    cargo test

# Install rustscale binary to ~/.local/bin/tailscale (warns on name conflict)
install: release
    #!/usr/bin/env bash
    set -euo pipefail
    target_dir="${CARGO_TARGET_DIR:-target}"
    install_dir="${HOME}/.local/bin"
    mkdir -p "${install_dir}"
    binary="${install_dir}/tailscale"
    if [[ -e "${binary}" && ! -L "${binary}" ]]; then
      echo "WARNING: ${binary} already exists as a regular file." >&2
      echo "         Skipping install to avoid overwriting the real Tailscale CLI." >&2
      echo "         To force: cp target/release/tailscale ${binary}" >&2
      exit 0
    fi
    if command -v tailscale >/dev/null 2>&1; then
      existing="$(command -v tailscale)"
      if [[ "${existing}" != "${binary}" ]]; then
        echo "WARNING: 'tailscale' resolves to ${existing}" >&2
        echo "         Installing to ${binary} may shadow the real Tailscale CLI" >&2
        echo "         if ~/.local/bin appears first in PATH." >&2
      fi
    fi
    install -m 755 "${target_dir}/release/tailscale" "${binary}"
    echo "Installed: ${binary}"

docker-build:
    docker build -f config/Dockerfile -t rustscale .

# Start rustscale via Docker Compose (container named tailscale-mcp to avoid conflict)
docker-up:
    docker compose up -d

# Stop the rustscale Docker Compose stack
docker-down:
    docker compose down

# Restart the rustscale container
restart:
    docker compose restart tailscale-mcp

logs:
    docker compose logs -f tailscale-mcp

health:
    curl -sf http://localhost:7575/health | jq .

# Repair: stop the container, rebuild, and restart
repair:
    #!/usr/bin/env bash
    set -euo pipefail
    docker compose down || true
    cargo build --release
    target_dir="${CARGO_TARGET_DIR:-target}"
    install -m 755 "${target_dir}/release/tailscale" bin/tailscale 2>/dev/null || true
    docker compose up -d --build
    echo "Repair complete — container restarted"

setup:
    cp -n .env.example .env || true

gen-token:
    openssl rand -hex 32


validate-skills:
    #!/usr/bin/env bash
    set -euo pipefail
    found=0
    for dir in plugins/tailscale/skills/*; do
      [[ -d "$dir" ]] || continue
      found=1
      test -f "$dir/SKILL.md" || { echo "MISSING: $dir/SKILL.md"; exit 1; }
      grep -q '^name:' "$dir/SKILL.md" || { echo "MISSING name: $dir/SKILL.md"; exit 1; }
      grep -q '^description:' "$dir/SKILL.md" || { echo "MISSING description: $dir/SKILL.md"; exit 1; }
    done
    [[ "$found" -eq 1 ]] || { echo "MISSING: plugins/tailscale/skills/*"; exit 1; }
    echo "OK"

# Run the mcporter integration smoke-test (requires running server + mcporter in PATH)
test-mcporter:
    #!/usr/bin/env bash
    set -euo pipefail
    bash tests/mcporter/test-tools.sh

# Generate a standalone CLI for this server (requires running server; HTTP-only transport)
generate-cli:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Server must be running on port 7575 (run 'just dev' first)"
    echo "Generated CLI embeds your OAuth token — do not commit or share"
    mkdir -p dist dist/.cache
    current_hash=$(timeout 10 curl -sf \
      -H "Authorization: Bearer $MCP_TOKEN" \
      -H "Accept: application/json, text/event-stream" \
      http://localhost:7575/mcp/tools/list 2>/dev/null | sha256sum | cut -d' ' -f1 || echo "nohash")
    cache_file="dist/.cache/rustscale-cli.schema_hash"
    if [[ -f "$cache_file" ]] && [[ "$(cat "$cache_file")" == "$current_hash" ]] && [[ -f "dist/rustscale-cli" ]]; then
      echo "SKIP: tool schema unchanged — use existing dist/rustscale-cli"
      exit 0
    fi
    timeout 30 mcporter generate-cli \
      --command http://localhost:7575/mcp \
      --header "Authorization: Bearer $MCP_TOKEN" \
      --name rustscale-cli \
      --output dist/rustscale-cli
    printf '%s' "$current_hash" > "$cache_file"
    echo "Generated dist/rustscale-cli (requires bun at runtime)"

clean:
    cargo clean
    rm -rf .cache/

# Linux only — Windows would need .exe binaries; requires git lfs install
build-plugin: release
    #!/bin/sh
    set -eu
    target_dir="${CARGO_TARGET_DIR:-target}"
    if [ ! -x "$target_dir/release/tailscale" ] && [ -x ".cache/cargo/release/tailscale" ]; then
      target_dir=".cache/cargo"
    fi
    install -m 755 "$target_dir/release/tailscale" bin/tailscale

# Publish: bump version, tag, push (triggers crates.io + Docker publish)
publish bump="patch":
    #!/usr/bin/env bash
    set -euo pipefail
    [ "$(git branch --show-current)" = "main" ] || { echo "Switch to main first"; exit 1; }
    [ -z "$(git status --porcelain)" ] || { echo "Commit or stash changes first"; exit 1; }
    git pull origin main
    CURRENT=$(grep -m1 "^version" Cargo.toml | sed "s/.*\"\(.*\)\".*/\1/")
    IFS="." read -r major minor patch <<< "$CURRENT"
    case "{{bump}}" in
      major) major=$((major+1)); minor=0; patch=0 ;;
      minor) minor=$((minor+1)); patch=0 ;;
      patch) patch=$((patch+1)) ;;
      *) echo "Usage: just publish [major|minor|patch]"; exit 1 ;;
    esac
    NEW="${major}.${minor}.${patch}"
    echo "Version: ${CURRENT} → ${NEW}"
    sed -i "s/^version = \"${CURRENT}\"/version = \"${NEW}\"/" Cargo.toml
    cargo check 2>/dev/null || true
    for f in .claude-plugin/plugin.json .codex-plugin/plugin.json gemini-extension.json; do
      [ -f "$f" ] && python3 -c 'import json,sys; p=sys.argv[1]; v=sys.argv[2]; d=json.load(open(p)); d["version"]=v; json.dump(d,open(p,"w"),indent=2); open(p,"a").write("\n")' "$f" "${NEW}"
    done
    git add -A && git commit -m "release: v${NEW}" && git tag "v${NEW}" && git push origin main --tags
    echo "Tagged v${NEW} — publish workflow will run automatically"

# Refresh local reference documentation (crawls + repomix)
refresh-docs:
    bash scripts/refresh-docs.sh

# Refresh docs — repomix packs only (no crawl)
refresh-docs-repomix:
    bash scripts/refresh-docs.sh --skip-crawl

# Refresh docs — crawl only (no repomix)
refresh-docs-crawl:
    bash scripts/refresh-docs.sh --skip-repomix

# Dry-run: print what would be refreshed
refresh-docs-dry:
    bash scripts/refresh-docs.sh --dry-run
