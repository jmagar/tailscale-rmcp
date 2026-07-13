#!/usr/bin/env bash
# SessionStart / ConfigChange hook for the Tailscale plugin.
set -euo pipefail

binary="${TAILSCALE_RMCP_BIN:-rtailscale}"

if ! command -v "${binary}" >/dev/null 2>&1; then
  printf 'tailscale plugin setup: rtailscale is not installed or not on PATH.\n' >&2
  printf 'Install rtailscale separately, then run: rtailscale setup\n' >&2
  exit 0
fi

exec "${binary}" setup plugin-hook "$@"
