#!/usr/bin/env bash
# Claude Code plugin setup hook. Keep service setup owned by the tailscale binary.
set -euo pipefail

: "${CLAUDE_PLUGIN_ROOT:=$(cd "$(dirname "$0")/.." && pwd)}"
: "${CLAUDE_PLUGIN_DATA:=${HOME}/.claude/plugins/data/tailscale-jmagar-lab}"

reject_unsafe_value() {
  local name="$1" value="${2:-}"
  if [[ "${value}" == *$'\n'* || "${value}" == *$'\r'* ]]; then
    printf 'tailscale plugin setup: %s must not contain newlines\n' "${name}" >&2
    exit 2
  fi
}

existing_env_value() {
  local key="$1" file value
  for file in "${CLAUDE_PLUGIN_DATA}/.env"; do
    [[ -f "${file}" ]] || continue
    value="$(awk -F= -v key="${key}" '$1 == key {print substr($0, index($0, "=") + 1); exit}' "${file}")"
    [[ -n "${value}" ]] && { printf '%s\n' "${value}"; return 0; }
  done
  return 0
}

export_option() {
  local env_name="$1" option_name="$2" fallback_key="${3:-}" value
  value="$(printenv "${option_name}" || true)"
  if [[ -z "${value}" && -n "${fallback_key}" ]]; then
    value="$(existing_env_value "${fallback_key}")"
  fi
  reject_unsafe_value "${option_name}" "${value}"
  [[ -n "${value}" ]] || return 0
  export "${env_name}=${value}"
}

ensure_tailscale_binary() {
  # Resolve the bundled rustscale MCP binary by absolute path. Do NOT rely on
  # `command -v tailscale` — that resolves to the system Tailscale CLI
  # (/usr/bin/tailscale), a different program with no `setup` subcommand.
  local bundled="${CLAUDE_PLUGIN_ROOT}/bin/tailscale"
  if [[ -x "${bundled}" ]]; then
    TAILSCALE_BIN="${bundled}"
    return 0
  fi

  printf 'tailscale plugin setup: bundled tailscale MCP binary not found at %s\n' "${bundled}" >&2
  printf '  → run: just build-plugin   (builds the release binary into plugins/tailscale/bin/)\n' >&2
  exit 1
}

main() {
  mkdir -p "${CLAUDE_PLUGIN_DATA}"
  chmod 700 "${CLAUDE_PLUGIN_DATA}" 2>/dev/null || true
  export TAILSCALE_MCP_HOME="${CLAUDE_PLUGIN_DATA}"

  export_option TAILSCALE_MCP_TOKEN CLAUDE_PLUGIN_OPTION_API_TOKEN TAILSCALE_MCP_TOKEN
  export_option TAILSCALE_MCP_NO_AUTH CLAUDE_PLUGIN_OPTION_NO_AUTH TAILSCALE_MCP_NO_AUTH
  export_option TAILSCALE_MCP_HOST CLAUDE_PLUGIN_OPTION_MCP_HOST TAILSCALE_MCP_HOST
  export_option TAILSCALE_MCP_PORT CLAUDE_PLUGIN_OPTION_MCP_PORT TAILSCALE_MCP_PORT
  export_option TAILSCALE_MCP_AUTH_MODE CLAUDE_PLUGIN_OPTION_AUTH_MODE TAILSCALE_MCP_AUTH_MODE
  export_option TAILSCALE_MCP_PUBLIC_URL CLAUDE_PLUGIN_OPTION_PUBLIC_URL TAILSCALE_MCP_PUBLIC_URL
  export_option TAILSCALE_MCP_GOOGLE_CLIENT_ID CLAUDE_PLUGIN_OPTION_GOOGLE_CLIENT_ID TAILSCALE_MCP_GOOGLE_CLIENT_ID
  export_option TAILSCALE_MCP_GOOGLE_CLIENT_SECRET CLAUDE_PLUGIN_OPTION_GOOGLE_CLIENT_SECRET TAILSCALE_MCP_GOOGLE_CLIENT_SECRET
  export_option TAILSCALE_MCP_AUTH_ADMIN_EMAIL CLAUDE_PLUGIN_OPTION_AUTH_ADMIN_EMAIL TAILSCALE_MCP_AUTH_ADMIN_EMAIL
  export_option TAILSCALE_API_KEY CLAUDE_PLUGIN_OPTION_TAILSCALE_API_KEY TAILSCALE_API_KEY
  export_option TAILSCALE_TAILNET CLAUDE_PLUGIN_OPTION_TAILSCALE_TAILNET TAILSCALE_TAILNET
  export_option TAILSCALE_ALLOW_DESTRUCTIVE CLAUDE_PLUGIN_OPTION_ALLOW_DESTRUCTIVE TAILSCALE_ALLOW_DESTRUCTIVE

  ensure_tailscale_binary
  "${TAILSCALE_BIN}" setup plugin-hook "$@"
}

main "$@"
