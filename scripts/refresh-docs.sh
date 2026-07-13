#!/usr/bin/env bash
# refresh-docs.sh — Refresh reference docs for tailscale-rmcp (Tailscale MCP)
# Pattern: §38 — Crawls Tailscale API docs + packs Tailscale repos
# Usage: scripts/refresh-docs.sh [--dry-run] [--skip-crawl] [--skip-repomix]
#
# Crawled:  https://tailscale.com/api   https://modelcontextprotocol.io
# Repomix:  tailscale/tailscale (filtered to API paths), modelcontextprotocol/rust-sdk
set -Eeuo pipefail; IFS=$'\n\t'
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
ROOT_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd -P)"
REF_DIR="$ROOT_DIR/docs/references"; CHANGES_FILE="$REF_DIR/CHANGES.md"
AXON_OUTPUT_DIR="${AXON_OUTPUT_DIR:-$HOME/.axon/output}"
DRY_RUN=false; SKIP_CRAWL=false; SKIP_REPOMIX=false
while [[ $# -gt 0 ]]; do case "$1" in
  --dry-run) DRY_RUN=true;shift ;; --skip-crawl) SKIP_CRAWL=true;shift ;;
  --skip-repomix) SKIP_REPOMIX=true;shift ;; -h|--help) echo "Usage: scripts/refresh-docs.sh [--dry-run] [--skip-crawl] [--skip-repomix]";exit 0 ;;
  *) echo "ERROR: unknown: $1" >&2;exit 2 ;; esac; done
[[ "$SKIP_CRAWL" == true && "$SKIP_REPOMIX" == true ]] && { echo "ERROR: cannot combine" >&2;exit 2; }
log() { printf '[refresh-docs] %s\n' "$*"; }
refresh_scope() { if [[ "$SKIP_CRAWL" == true ]]; then printf repomix-only; elif [[ "$SKIP_REPOMIX" == true ]]; then printf crawl-only; else printf full; fi; }
require_cmd() { command -v "$1" >/dev/null 2>&1 || { echo "ERROR: $1 not found" >&2;exit 1; }; }
make_tmpdir() { mktemp -d "${TMPDIR:-/tmp}/tailscale-rmcp-refresh-docs.XXXXXX"; }
atomic_replace_dir() {
  local src="$1" dst="$2" parent backup; parent="$(dirname -- "$dst")"; mkdir -p "$parent"
  backup="$(mktemp -d "$parent/.$(basename "$dst").backup.XXXXXX")"; rmdir "$backup"
  [[ -e "$dst" ]] && mv -- "$dst" "$backup"
  if mv -- "$src" "$dst"; then rm -rf -- "$backup"; else [[ -e "$backup" ]] && mv -- "$backup" "$dst"; return 1; fi
}
copy_job_output_to_layout() {
  local sd="$1" td="$2" tmp
  [[ -f "$sd/manifest.jsonl" ]] || { echo "ERROR: missing manifest" >&2;return 1; }
  [[ -d "$sd/markdown" ]]       || { echo "ERROR: missing markdown" >&2;return 1; }
  tmp="$(make_tmpdir)"; cp -a "$sd/." "$tmp/"; atomic_replace_dir "$tmp" "$td"
}
newest_domain_run() {
  local dd="$AXON_OUTPUT_DIR/domains/$1"; [[ -d "$dd" ]] || return 1
  find "$dd" -mindepth 1 -maxdepth 1 -type d -printf '%T@ %p\n' | sort -nr | awk 'NR==1{$1="";sub(/^ /,"");print}'
}
crawl_docs() {
  local url="$1" domain="$2" tr="$3" td="$REF_DIR/$3" out job sd
  log "crawl $url -> docs/references/$tr"; [[ "$DRY_RUN" == true ]] && return 0; require_cmd axon
  out="$(axon crawl "$url" --wait true --yes 2>&1)"; printf '%s\n' "$out"
  job="$(awk '/^Job ID:/{print $3}' <<<"$out" | tail -1)"
  if [[ -n "$job" && -d "$AXON_OUTPUT_DIR/domains/$domain/$job" ]]; then sd="$AXON_OUTPUT_DIR/domains/$domain/$job"; else sd="$(newest_domain_run "$domain")"; fi
  [[ -n "$sd" && -d "$sd" ]] || { echo "ERROR: no Axon output for $domain" >&2;return 1; }
  copy_job_output_to_layout "$sd" "$td"
}
repomix_command() {
  if [[ -n "${REPOMIX_BIN:-}" ]]; then "$REPOMIX_BIN" "$@"; elif command -v repomix >/dev/null 2>&1; then repomix "$@"; else require_cmd npx; npx --yes repomix "$@"; fi
}
pack_repo() {
  local remote="$1" tr="$2" inc="${3:-}" ign="${4:-}" tf="$REF_DIR/$2" td tmp_file
  log "pack $remote -> docs/references/$tr"; [[ "$DRY_RUN" == true ]] && return 0
  td="$(make_tmpdir)"; tmp_file="$td/out.xml"
  local args=(--remote "$remote" --style xml --output "$tmp_file" --top-files-len 10)
  [[ -n "$inc" ]] && args+=(--include "$inc"); [[ -n "$ign" ]] && args+=(--ignore "$ign")
  repomix_command "${args[@]}"
  [[ -s "$tmp_file" ]] || { echo "ERROR: no output for $remote" >&2;rm -rf -- "$td";return 1; }
  mkdir -p "$(dirname -- "$tf")"; mv -- "$tmp_file" "$tf"; rm -rf -- "$td"
}
write_index() {
  local t=0 m=0
  [[ -d "$REF_DIR/tailscale/docs" ]] && t="$(find "$REF_DIR/tailscale/docs" -type f|wc -l|tr -d ' ')"
  [[ -d "$REF_DIR/mcp/docs"      ]] && m="$(find "$REF_DIR/mcp/docs"      -type f|wc -l|tr -d ' ')"
  cat > "$REF_DIR/INDEX.md" <<EOF
# Reference Index — tailscale-rmcp (Tailscale MCP)
| Path | Contents | Source |
|---|---|---|
| \`tailscale/docs/\`  | Tailscale API reference crawl      | tailscale.com/api |
| \`tailscale/repos/\` | Tailscale Go source (API paths)    | tailscale/tailscale |
| \`mcp/docs/\`        | MCP protocol docs                  | modelcontextprotocol.io |
| \`mcp/repos/\`       | MCP Rust SDK                       | modelcontextprotocol/* |
## Tailscale API Notes
- Base URL: https://api.tailscale.com/api/v2
- Auth: Authorization: Bearer <api_key>
- Tailnet: your org name or "-" for personal
_Updated: $(date -u +%Y-%m-%dT%H:%M:%SZ)_
EOF
}
snapshot_references() {
  [[ ! -d "$REF_DIR" ]] && { :>"$1";return 0; }
  (cd "$REF_DIR";find . -type f ! -path './CHANGES.md' -print0|sort -z|xargs -0 -r sha256sum|sed 's#  \./#  #') > "$1"
}
snapshot_paths() { awk '{$1="";sub(/^  /,"");print}' "$1"; }
ensure_changes_file() {
  mkdir -p "$REF_DIR"; [[ -f "$CHANGES_FILE" ]] && return 0
  printf -- '---\ntitle: Reference Refresh Log — tailscale-rmcp\ncreated_at: %s\n---\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$CHANGES_FILE"
}
append_changes_log() {
  ensure_changes_file
  { printf '\n## %s\n\n- scope: `%s`\n- summary: `%s added, %s modified, %s removed`\n' \
      "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$(refresh_scope)" "$4" "$5" "$6"; } >> "$CHANGES_FILE"
}
summarize_reference_changes() {
  local b="$1" a="$2" td; td="$(make_tmpdir)"
  snapshot_paths "$b"|sort>"$td/b"; snapshot_paths "$a"|sort>"$td/a"
  comm -13 "$td/b" "$td/a">"$td/add"; comm -23 "$td/b" "$td/a">"$td/rm"; comm -12 "$td/b" "$td/a">"$td/com"; :>"$td/mod"
  while IFS= read -r p; do
    [[ "$(grep -F "  $p" "$b"|cut -d' ' -f1)" != "$(grep -F "  $p" "$a"|cut -d' ' -f1)" ]] && printf '%s\n' "$p">>"$td/mod"
  done <"$td/com"
  local ac rc mc; ac="$(wc -l<"$td/add"|tr -d ' ')"; rc="$(wc -l<"$td/rm"|tr -d ' ')"; mc="$(wc -l<"$td/mod"|tr -d ' ')"
  log "change summary: $ac added, $mc modified, $rc removed"
  append_changes_log "$td/add" "$td/mod" "$td/rm" "$ac" "$mc" "$rc"; rm -rf -- "$td"
}
main() {
  local sd bs as
  if [[ "$DRY_RUN" != true ]]; then sd="$(make_tmpdir)"; bs="$sd/before.sha256"; as="$sd/after.sha256"; snapshot_references "$bs"; fi
  mkdir -p "$REF_DIR/tailscale/docs" "$REF_DIR/tailscale/repos" "$REF_DIR/mcp/docs" "$REF_DIR/mcp/repos"
  if [[ "$SKIP_CRAWL" != true ]]; then
    crawl_docs "https://tailscale.com/api" || log "WARN: tailscale docs crawl failed, continuing"          "tailscale.com"           "tailscale/docs"
    crawl_docs "https://modelcontextprotocol.io"    "modelcontextprotocol.io" "mcp/docs" || log "WARN: mcp docs crawl failed, continuing"
  fi
  if [[ "$SKIP_REPOMIX" != true ]]; then
    # Tailscale Go source — filter to API-relevant paths (client/types, not full binary)
    pack_repo "tailscale/tailscale"                "tailscale/repos/tailscale-tailscale.xml" \
      "client/**,types/**,tailcfg/**,ipn/**" "**/*_test.go,vendor/**"
    pack_repo "modelcontextprotocol/rust-sdk"      "mcp/repos/modelcontextprotocol-rust-sdk.xml"
    pack_repo "modelcontextprotocol/registry"      "mcp/repos/modelcontextprotocol-registry.xml"
  fi
  if [[ "$DRY_RUN" != true ]]; then
    write_index; snapshot_references "$as"; summarize_reference_changes "$bs" "$as"; rm -rf -- "$sd"
  fi
  log "done"
}
main "$@"
