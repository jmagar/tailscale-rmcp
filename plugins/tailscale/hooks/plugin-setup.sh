#!/usr/bin/env bash
# SessionStart / ConfigChange hook — deploys or connects rustscale (Tailscale MCP) based on userConfig
set -euo pipefail

# When invoked directly (e.g. for debugging), the plugin runtime vars are absent.
# Derive CLAUDE_PLUGIN_ROOT from the script's own location.
: "${CLAUDE_PLUGIN_ROOT:=$(cd "$(dirname "$0")/.." && pwd)}"
: "${CLAUDE_PLUGIN_DATA:=${HOME}/.claude/plugins/data/tailscale-jmagar-lab}"

# ── Helpers ───────────────────────────────────────────────────────────────────

existing_env_value() {
  local key="$1"
  local file
  local value
  for file in "${CLAUDE_PLUGIN_DATA}/.env"; do
    [[ -f "${file}" ]] || continue
    value="$(awk -F= -v key="${key}" '$1 == key {print substr($0, index($0, "=") + 1); exit}' "${file}")"
    if [[ -n "${value}" ]]; then
      printf '%s\n' "${value}"
      return 0
    fi
  done
  return 0
}

validate_port_value() {
  local name="$1" value="$2"
  if ! [[ "${value}" =~ ^[0-9]+$ ]] || (( value < 1 || value > 65535 )); then
    echo "ERROR: ${name} must be a TCP/UDP port number (1-65535), got: ${value}" >&2
    exit 1
  fi
}

mcp_host_is_loopback() {
  case "$1" in
    127.*|::1) return 0 ;;
    *) return 1 ;;
  esac
}

strip_trailing_mcp_path() {
  local url="${1%/}"
  if [[ "${url}" == */mcp ]]; then
    url="${url%/mcp}"
  fi
  printf '%s\n' "${url}"
}

derive_public_url() {
  if [[ -n "${PUBLIC_URL}" ]]; then
    strip_trailing_mcp_path "${PUBLIC_URL}"
    return
  fi
  if [[ "${SERVER_URL}" == https://* ]]; then
    strip_trailing_mcp_path "${SERVER_URL}"
  fi
}

codex_oauth_callback_url() {
  local config="${HOME}/.codex/config.toml"
  [[ -f "${config}" ]] || return 0
  awk -F= '
    $1 ~ /^[[:space:]]*mcp_oauth_callback_url[[:space:]]*$/ {
      value = $2
      sub(/^[[:space:]]*"/, "", value)
      sub(/"[[:space:]]*$/, "", value)
      print value
      exit
    }
  ' "${config}"
}

append_csv_unique() {
  local csv="$1"
  local value="$2"
  [[ -n "${value}" ]] || { printf '%s\n' "${csv}"; return; }

  local existing
  IFS=',' read -r -a existing <<< "${csv}"
  for item in "${existing[@]}"; do
    item="${item#"${item%%[![:space:]]*}"}"
    item="${item%"${item##*[![:space:]]}"}"
    if [[ "${item}" == "${value}" ]]; then
      printf '%s\n' "${csv}"
      return
    fi
  done

  if [[ -n "${csv}" ]]; then
    printf '%s,%s\n' "${csv}" "${value}"
  else
    printf '%s\n' "${value}"
  fi
}

# ── Seed token from existing env (so redeploy doesn't fail without plugin vars) ──
NO_AUTH="${CLAUDE_PLUGIN_OPTION_NO_AUTH:-$(existing_env_value NO_AUTH)}"
NO_AUTH="${NO_AUTH:-false}"
NO_AUTH="$(printf '%s' "${NO_AUTH}" | tr '[:upper:]' '[:lower:]')"

AUTH_MODE="${CLAUDE_PLUGIN_OPTION_AUTH_MODE:-$(existing_env_value TAILSCALE_MCP_AUTH_MODE)}"
AUTH_MODE="${AUTH_MODE:-bearer}"
AUTH_MODE="$(printf '%s' "${AUTH_MODE}" | tr '[:upper:]' '[:lower:]')"

if [[ "${NO_AUTH}" != "true" && -z "${CLAUDE_PLUGIN_OPTION_API_TOKEN:-}" ]]; then
  _tok="$(existing_env_value TAILSCALE_MCP_TOKEN)"
  [[ -n "${_tok}" ]] && CLAUDE_PLUGIN_OPTION_API_TOKEN="${_tok}"
  unset _tok
fi

# ── Config from userConfig ─────────────────────────────────────────────────────
USE_DOCKER="${CLAUDE_PLUGIN_OPTION_USE_DOCKER:-false}"
API_TOKEN="${CLAUDE_PLUGIN_OPTION_API_TOKEN:-}"
SERVER_URL="${CLAUDE_PLUGIN_OPTION_SERVER_URL:-http://localhost:7575}"
MCP_HOST="${CLAUDE_PLUGIN_OPTION_MCP_HOST:-0.0.0.0}"
MCP_PORT="${CLAUDE_PLUGIN_OPTION_MCP_PORT:-7575}"
validate_port_value TAILSCALE_MCP_PORT "${MCP_PORT}"

TAILSCALE_API_KEY="${CLAUDE_PLUGIN_OPTION_TAILSCALE_API_KEY:-$(existing_env_value TAILSCALE_API_KEY)}"
TAILSCALE_TAILNET="${CLAUDE_PLUGIN_OPTION_TAILSCALE_TAILNET:-$(existing_env_value TAILSCALE_TAILNET)}"
TAILSCALE_TAILNET="${TAILSCALE_TAILNET:--}"
ALLOW_DESTRUCTIVE="${CLAUDE_PLUGIN_OPTION_ALLOW_DESTRUCTIVE:-$(existing_env_value TAILSCALE_ALLOW_DESTRUCTIVE)}"
ALLOW_DESTRUCTIVE="${ALLOW_DESTRUCTIVE:-false}"
ALLOW_DESTRUCTIVE="$(printf '%s' "${ALLOW_DESTRUCTIVE}" | tr '[:upper:]' '[:lower:]')"

PUBLIC_URL="${CLAUDE_PLUGIN_OPTION_PUBLIC_URL:-$(existing_env_value TAILSCALE_MCP_PUBLIC_URL)}"
GOOGLE_CLIENT_ID="${CLAUDE_PLUGIN_OPTION_GOOGLE_CLIENT_ID:-$(existing_env_value TAILSCALE_MCP_GOOGLE_CLIENT_ID)}"
GOOGLE_CLIENT_SECRET="${CLAUDE_PLUGIN_OPTION_GOOGLE_CLIENT_SECRET:-$(existing_env_value TAILSCALE_MCP_GOOGLE_CLIENT_SECRET)}"
AUTH_ADMIN_EMAIL="${CLAUDE_PLUGIN_OPTION_AUTH_ADMIN_EMAIL:-$(existing_env_value TAILSCALE_MCP_AUTH_ADMIN_EMAIL)}"
AUTH_ALLOWED_REDIRECT_URIS="${CLAUDE_PLUGIN_OPTION_AUTH_ALLOWED_REDIRECT_URIS:-$(existing_env_value TAILSCALE_MCP_AUTH_ALLOWED_REDIRECT_URIS)}"

# Require Tailscale API key
if [[ -z "${TAILSCALE_API_KEY}" ]]; then
  echo "ERROR: tailscale_api_key is required — set it in plugin userConfig or TAILSCALE_API_KEY env var" >&2
  exit 1
fi

# Require a token unless no_auth is true
if [[ "${NO_AUTH}" != "true" && -z "${API_TOKEN}" ]]; then
  if ! mcp_host_is_loopback "${MCP_HOST}"; then
    echo "ERROR: api_token is required unless no_auth is true or MCP binds to loopback" >&2
    exit 1
  fi
fi

# ── Paths ─────────────────────────────────────────────────────────────────────
ENV_FILE="${CLAUDE_PLUGIN_DATA}/.env"
UNIT_FILE="${HOME}/.config/systemd/user/tailscale-mcp.service"
COMPOSE_DIR="${CLAUDE_PLUGIN_DATA}"
COMPOSE_FILE="${COMPOSE_DIR}/docker-compose.yml"

# ── OAuth env block ───────────────────────────────────────────────────────────
oauth_env_block() {
  if [[ "${NO_AUTH}" == "true" ]]; then
    return 0
  fi
  if [[ "${AUTH_MODE}" != "bearer" && "${AUTH_MODE}" != "oauth" ]]; then
    echo "ERROR: auth_mode must be bearer or oauth" >&2
    return 1
  fi
  if [[ "${AUTH_MODE}" != "oauth" ]]; then
    return 0
  fi

  local public_url
  public_url="$(derive_public_url)"
  if [[ -z "${public_url}" ]]; then
    echo "ERROR: OAuth mode requires public_url or an https server_url" >&2
    return 1
  fi
  if [[ -z "${GOOGLE_CLIENT_ID}" || -z "${GOOGLE_CLIENT_SECRET}" || -z "${AUTH_ADMIN_EMAIL}" ]]; then
    echo "ERROR: OAuth mode requires google_client_id, google_client_secret, and auth_admin_email" >&2
    return 1
  fi

  local redirects="${AUTH_ALLOWED_REDIRECT_URIS}"
  redirects="$(append_csv_unique "${redirects}" "https://claude.ai/api/mcp/auth_callback")"
  redirects="$(append_csv_unique "${redirects}" "https://claudeai.ai/api/mcp/auth_callback")"

  local codex_callback
  codex_callback="$(codex_oauth_callback_url)"
  if [[ -n "${codex_callback}" ]]; then
    redirects="$(append_csv_unique "${redirects}" "${codex_callback}")"
  fi

  cat << EOF
TAILSCALE_MCP_AUTH_MODE=oauth
TAILSCALE_MCP_PUBLIC_URL=${public_url}
TAILSCALE_MCP_GOOGLE_CLIENT_ID=${GOOGLE_CLIENT_ID}
TAILSCALE_MCP_GOOGLE_CLIENT_SECRET=${GOOGLE_CLIENT_SECRET}
TAILSCALE_MCP_AUTH_ADMIN_EMAIL=${AUTH_ADMIN_EMAIL}
TAILSCALE_MCP_AUTH_ALLOWED_REDIRECT_URIS=${redirects}
EOF
}

# ── Write .env (returns 0=changed, 1=unchanged, 2=error) ─────────────────────
write_env() {
  mkdir -p "${CLAUDE_PLUGIN_DATA}"

  local new_env
  new_env=$(cat << EOF
TAILSCALE_API_KEY=${TAILSCALE_API_KEY}
TAILSCALE_TAILNET=${TAILSCALE_TAILNET}
TAILSCALE_ALLOW_DESTRUCTIVE=${ALLOW_DESTRUCTIVE}
TAILSCALE_MCP_HOST=${MCP_HOST}
TAILSCALE_MCP_PORT=${MCP_PORT}
NO_AUTH=${NO_AUTH}
EOF
)

  if [[ "${NO_AUTH}" != "true" && -n "${API_TOKEN}" ]]; then
    new_env="${new_env}
TAILSCALE_MCP_TOKEN=${API_TOKEN}"
  fi

  local auth_block
  if ! auth_block="$(oauth_env_block)"; then
    return 2
  fi
  [[ -n "${auth_block}" ]] && new_env="${new_env}
${auth_block}"

  if [[ "${USE_DOCKER}" == "true" ]]; then
    new_env="${new_env}
TAILSCALE_UID=$(id -u)
TAILSCALE_GID=$(id -g)"
  fi

  if [[ -f "${ENV_FILE}" ]] && diff -q <(printf '%s\n' "${new_env}") "${ENV_FILE}" >/dev/null 2>&1; then
    return 1  # unchanged
  fi

  printf '%s\n' "${new_env}" > "${ENV_FILE}"
  chmod 600 "${ENV_FILE}"
  return 0  # changed
}

ensure_env_written() {
  local rc
  write_env; rc=$?
  if [[ "${rc}" -eq 0 || "${rc}" -eq 1 ]]; then
    return 0
  fi
  return "${rc}"
}

# ── Systemd deployment ─────────────────────────────────────────────────────────
setup_systemd() {
  mkdir -p "${HOME}/.config/systemd/user"

  if [[ ! -x "${CLAUDE_PLUGIN_ROOT}/bin/tailscale" ]]; then
    echo "ERROR: tailscale binary not found at ${CLAUDE_PLUGIN_ROOT}/bin/tailscale" >&2
    return 1
  fi

  # Port conflict check — skip when service is already running
  local service_running=false
  if systemctl --user is-active --quiet tailscale-mcp.service 2>/dev/null; then
    service_running=true
  fi
  if [[ "${service_running}" == "false" ]]; then
    for port_proto in "${MCP_PORT}/tcp"; do
      local port="${port_proto%%/*}" proto="${port_proto##*/}"
      if ss -"${proto:0:1}"lnp "sport = :${port}" 2>/dev/null | awk 'NR>1 && NF>0' | grep -q .; then
        echo "ERROR: port ${port}/${proto} is already in use — cannot start tailscale-mcp" >&2
        return 1
      fi
    done
  fi

  # Stop docker container if switching modes
  if [[ -f "${COMPOSE_FILE}" ]] && command -v docker >/dev/null 2>&1; then
    if (cd "${COMPOSE_DIR}" && docker compose ps --quiet tailscale-mcp 2>/dev/null | grep -q .); then
      echo "tailscale-mcp: stopping existing docker container before systemd cutover"
      (cd "${COMPOSE_DIR}" && docker compose down)
    fi
  fi

  local new_unit
  new_unit=$(cat << EOF
[Unit]
Description=tailscale-mcp server (rustscale)
After=network.target

[Service]
ExecStart=${CLAUDE_PLUGIN_ROOT}/bin/tailscale serve mcp
EnvironmentFile=${ENV_FILE}
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
EOF
)

  local unit_changed=false
  if ! diff -q <(printf '%s\n' "${new_unit}") "${UNIT_FILE}" >/dev/null 2>&1; then
    printf '%s\n' "${new_unit}" > "${UNIT_FILE}"
    unit_changed=true
  fi

  ensure_env_written

  if [[ "${unit_changed}" == "true" ]]; then
    systemctl --user daemon-reload
    systemctl --user enable --now tailscale-mcp
  else
    systemctl --user restart tailscale-mcp
  fi

  echo "tailscale-mcp: systemd service running on ${MCP_HOST}:${MCP_PORT}"
}

# ── Docker deployment ──────────────────────────────────────────────────────────
setup_docker() {
  mkdir -p "${COMPOSE_DIR}"

  if ! docker info >/dev/null 2>&1; then
    echo "ERROR: docker daemon is not reachable — is dockerd running?" >&2
    return 1
  fi

  # Port conflict check
  local container_running=false
  if [[ -f "${COMPOSE_FILE}" ]] && \
     docker compose -f "${COMPOSE_FILE}" ps --quiet tailscale-mcp 2>/dev/null | grep -q .; then
    container_running=true
  elif docker ps --filter 'name=^/tailscale-mcp$' --quiet 2>/dev/null | grep -q .; then
    container_running=true
  fi
  if [[ "${container_running}" == "false" ]]; then
    if ss -tlnp "sport = :${MCP_PORT}" 2>/dev/null | awk 'NR>1 && NF>0' | grep -q .; then
      echo "ERROR: port ${MCP_PORT}/tcp is already in use — cannot start tailscale-mcp" >&2
      return 1
    fi
  fi

  # Stop systemd unit if switching modes
  if systemctl --user list-unit-files tailscale-mcp.service >/dev/null 2>&1; then
    if systemctl --user is-active --quiet tailscale-mcp.service; then
      echo "tailscale-mcp: stopping existing systemd unit before docker cutover"
      systemctl --user stop tailscale-mcp.service
    fi
    if systemctl --user is-enabled --quiet tailscale-mcp.service 2>/dev/null; then
      systemctl --user disable tailscale-mcp.service >/dev/null 2>&1 || true
    fi
    if [[ -f "${UNIT_FILE}" ]]; then
      rm -f "${UNIT_FILE}"
      systemctl --user daemon-reload
    fi
  fi

  # Refresh compose file if plugin updated
  if ! diff -q "${CLAUDE_PLUGIN_ROOT}/../../../docker-compose.yml" "${COMPOSE_FILE}" >/dev/null 2>&1; then
    cp "${CLAUDE_PLUGIN_ROOT}/../../../docker-compose.yml" "${COMPOSE_FILE}"
  fi

  ensure_env_written

  cd "${COMPOSE_DIR}"

  local network_name="${DOCKER_NETWORK:-jakenet}"
  if ! docker network inspect "${network_name}" >/dev/null 2>&1; then
    echo "tailscale-mcp: creating docker network ${network_name}"
    docker network create "${network_name}"
  fi

  if [[ "${CLAUDE_PLUGIN_OPTION_BUILD_LOCAL:-false}" == "true" && -f "${CLAUDE_PLUGIN_ROOT}/../../../Cargo.toml" ]]; then
    (cd "${CLAUDE_PLUGIN_ROOT}/../../.." && docker compose build --no-cache tailscale-mcp)
  else
    docker compose pull --quiet tailscale-mcp 2>&1 || \
      echo "tailscale-mcp: pull failed; will try cached image" >&2
  fi

  if docker compose ps --quiet tailscale-mcp 2>/dev/null | grep -q .; then
    docker compose up -d --force-recreate --no-build
  else
    docker compose up -d --no-build
  fi

  echo "tailscale-mcp: docker container running on ${MCP_HOST}:${MCP_PORT}"
}

# ── Client-only mode: validate connectivity ────────────────────────────────────
validate_client() {
  if curl -sf "${SERVER_URL}/health" >/dev/null 2>&1; then
    echo "tailscale-mcp: connected to ${SERVER_URL}"
  else
    echo "WARNING: tailscale-mcp server at ${SERVER_URL} is not reachable" >&2
  fi
}

# ── Symlink binary into user PATH ──────────────────────────────────────────────
link_binary() {
  mkdir -p "${HOME}/.local/bin"
  if [[ -x "${CLAUDE_PLUGIN_ROOT}/bin/tailscale" ]]; then
    if [[ -e "${HOME}/.local/bin/tailscale" && ! -L "${HOME}/.local/bin/tailscale" ]]; then
      echo "WARNING: ${HOME}/.local/bin/tailscale already exists as a real file (not a symlink)." >&2
      echo "         Skipping symlink to avoid overwriting the Tailscale CLI binary." >&2
      return 0
    fi
    # Warn if a non-rustscale tailscale is in PATH
    if command -v tailscale >/dev/null 2>&1; then
      local current_ts
      current_ts="$(command -v tailscale)"
      if [[ "${current_ts}" != "${HOME}/.local/bin/tailscale" ]]; then
        echo "WARNING: 'tailscale' resolves to ${current_ts} — installing rustscale's binary at" >&2
        echo "         ${HOME}/.local/bin/tailscale may shadow the real Tailscale CLI if ~/.local/bin" >&2
        echo "         appears first in PATH." >&2
      fi
    fi
    ln -sf "${CLAUDE_PLUGIN_ROOT}/bin/tailscale" "${HOME}/.local/bin/tailscale"
  fi
}

# ── Main ──────────────────────────────────────────────────────────────────────
link_binary

if [[ "${USE_DOCKER}" == "true" ]]; then
  setup_docker
elif systemctl --user list-unit-files tailscale-mcp.service >/dev/null 2>&1 || \
     [[ -x "${CLAUDE_PLUGIN_ROOT}/bin/tailscale" ]]; then
  # Run as systemd service if binary exists
  if [[ -x "${CLAUDE_PLUGIN_ROOT}/bin/tailscale" ]]; then
    setup_systemd
  else
    validate_client
  fi
else
  validate_client
fi
