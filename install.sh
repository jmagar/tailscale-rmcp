#!/usr/bin/env bash
# install.sh — one-line installer for tailscale-rmcp (Tailscale MCP server)
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/jmagar/tailscale-rmcp/main/install.sh | bash
#   # or locally:
#   bash install.sh
#
# Environment controls:
#   INSTALL_DIR      Install directory (default: ~/.local/bin)
#   BINARY_NAME      Binary name to use (default: tailscale)
#                    Set to "tailscale-mcp" to avoid shadowing the real Tailscale CLI.
#   BUILD=1          Build from source instead of downloading a release binary
#   FORCE=1          Install even if a conflicting binary exists
#   TAILSCALE_RMCP_VERSION  Pin a specific release tag (e.g. v0.1.0)
#
# What it does:
#   1. Pre-flight checks (platform, tools, disk space, install dir, env vars, port)
#   2. Warns if a tailscale binary conflict is detected
#   3. Builds or downloads the tailscale-rmcp binary to INSTALL_DIR/BINARY_NAME
#   4. Creates ~/.tailscale-mcp/ data and log directories
#   5. Writes a starter ~/.tailscale-mcp/.env if one doesn't already exist
#   6. Runs `<binary> doctor` to validate the installation
#   7. Prints next steps
#
set -euo pipefail

INSTALL_DIR="${INSTALL_DIR:-${HOME}/.local/bin}"
BINARY_NAME="${BINARY_NAME:-tailscale}"
BINARY_PATH="${INSTALL_DIR}/${BINARY_NAME}"
DATA_DIR="${HOME}/.tailscale-mcp"
ENV_FILE="${DATA_DIR}/.env"
REPO="jmagar/tailscale-rmcp"
MCP_PORT="${TAILSCALE_MCP_PORT:-7575}"

# Colours
if [[ -t 1 ]]; then
  C_RESET='\033[0m'
  C_BOLD='\033[1m'
  C_GREEN='\033[0;32m'
  C_YELLOW='\033[0;33m'
  C_RED='\033[0;31m'
  C_CYAN='\033[0;36m'
else
  C_RESET='' C_BOLD='' C_GREEN='' C_YELLOW='' C_RED='' C_CYAN=''
fi

info()  { printf "${C_CYAN}[tailscale-rmcp]${C_RESET} %s\n"  "$*"; }
warn()  { printf "${C_YELLOW}[WARN]${C_RESET}      %s\n" "$*" >&2; }
error() { printf "${C_RED}[ERROR]${C_RESET}     %s\n"    "$*" >&2; }
ok()    { printf "${C_GREEN}[OK]${C_RESET}        %s\n"  "$*"; }

# ── Pre-flight checks ─────────────────────────────────────────────────────────

preflight() {
  local errors=0

  echo ""
  printf '%b%s%b\n' "${C_BOLD}" "Pre-flight checks..." "${C_RESET}"
  echo ""

  # 1. OS / arch
  local os arch
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  arch="$(uname -m)"
  case "${arch}" in
    x86_64)        arch_tag="x86_64" ;;
    aarch64|arm64) arch_tag="aarch64" ;;
    *)
      error "Unsupported architecture: ${arch}"
      (( errors++ )) || true
      arch_tag="${arch}"
      ;;
  esac
  case "${os}" in
    linux|darwin) ok "Platform: ${os}/${arch}" ;;
    *)
      error "Only Linux and macOS are supported (got: ${os})"
      (( errors++ )) || true
      ;;
  esac

  # 2. Required tools
  for cmd in curl tar; do
    if command -v "${cmd}" >/dev/null 2>&1; then
      ok "${cmd}: $(command -v "${cmd}")"
    else
      error "${cmd}: not found (required)"
      (( errors++ )) || true
    fi
  done

  # 3. Disk space (need at least 50 MB)
  local free_mb
  free_mb="$(df -k "${HOME}" | awk 'NR==2{printf "%d", $4/1024}')"
  if (( free_mb < 50 )); then
    error "Disk space: only ${free_mb} MB free in ${HOME} (need 50 MB)"
    (( errors++ )) || true
  else
    ok "Disk space: ${free_mb} MB free"
  fi

  # 4. Install directory writable
  if mkdir -p "${INSTALL_DIR}" && [[ -w "${INSTALL_DIR}" ]]; then
    ok "Install dir: ${INSTALL_DIR} (writable)"
  else
    error "Install dir: ${INSTALL_DIR} (not writable)"
    (( errors++ )) || true
  fi

  # 5. PATH check (warn only)
  if echo "${PATH}" | grep -q "${HOME}/.local/bin"; then
    ok "PATH: ~/.local/bin is present"
  else
    warn "PATH: ~/.local/bin not in PATH — add it to your shell profile"
  fi

  # 6. Binary name conflict check
  if [[ "${BINARY_NAME}" == "tailscale" ]]; then
    if command -v tailscale >/dev/null 2>&1; then
      local existing
      existing="$(command -v tailscale)"
      if [[ "${existing}" != "${BINARY_PATH}" ]]; then
        warn "BINARY NAME CONFLICT: 'tailscale' already resolves to: ${existing}"
        warn "  Installing tailscale-rmcp as 'tailscale' may shadow the real Tailscale CLI."
        warn "  To avoid conflict, re-run with: BINARY_NAME=tailscale-mcp bash install.sh"
        warn "  Or set FORCE=1 to install anyway (know what you're doing)."
      fi
    fi
  fi

  # 7. Required env vars (warn only — can be set post-install)
  if [[ -n "${TAILSCALE_API_KEY:-}" ]]; then
    ok "TAILSCALE_API_KEY: set"
  else
    warn "TAILSCALE_API_KEY: not set (required before running the server)"
  fi

  if [[ -n "${TAILSCALE_TAILNET:-}" ]]; then
    ok "TAILSCALE_TAILNET: ${TAILSCALE_TAILNET}"
  else
    warn "TAILSCALE_TAILNET: not set (will default to '-' for personal account)"
  fi

  # 8. Port availability (warn only)
  if ss -tlnp 2>/dev/null | awk '{print $4}' | grep -q ":${MCP_PORT}$"; then
    warn "Port ${MCP_PORT}: already in use (change TAILSCALE_MCP_PORT if needed)"
  else
    ok "Port ${MCP_PORT}: available"
  fi

  echo ""
  if (( errors > 0 )); then
    error "Pre-flight failed with ${errors} error(s). Fix them and re-run."
    return 1
  fi
  ok "Pre-flight passed — proceeding with install"
  echo ""
  return 0
}

# ── Binary conflict check (install-time gate) ─────────────────────────────────

warn_binary_conflict() {
  # If target path already has a regular file (not a symlink we placed), be careful.
  if [[ -e "${BINARY_PATH}" && ! -L "${BINARY_PATH}" ]]; then
    warn "${BINARY_PATH} already exists as a regular file (not a symlink)."
    warn "This may be the real Tailscale CLI binary."
    warn "Set FORCE=1 to overwrite, or use a different BINARY_NAME."
    return 1
  fi

  # If tailscale resolves to something else in PATH, warn about shadowing.
  if [[ "${BINARY_NAME}" == "tailscale" ]] && command -v tailscale >/dev/null 2>&1; then
    local existing
    existing="$(command -v tailscale)"
    if [[ "${existing}" != "${BINARY_PATH}" ]]; then
      warn "'tailscale' currently resolves to: ${existing}"
      warn "Installing tailscale-rmcp at ${BINARY_PATH} will shadow the real Tailscale CLI"
      warn "if ~/.local/bin appears before that directory in your PATH."
      warn ""
      warn "To avoid conflict, consider:"
      warn "  BINARY_NAME=tailscale-mcp bash install.sh"
    fi
  fi
  return 0
}

# ── Install via cargo (source build) ──────────────────────────────────────────

install_from_cargo() {
  if ! command -v cargo >/dev/null 2>&1; then
    error "cargo not found. Install Rust from https://rustup.rs/ and re-run."
    return 1
  fi

  info "Building tailscale-rmcp from source with cargo..."
  if [[ -f "Cargo.toml" && "$(basename "$(pwd)")" == "tailscale-rmcp" ]]; then
    # In-tree build
    cargo build --release --locked
    local target_dir="${CARGO_TARGET_DIR:-target}"
    cp "${target_dir}/release/tailscale" "${BINARY_PATH}"
  else
    # Remote build
    cargo install --git "https://github.com/${REPO}.git" --bin tailscale --root "${HOME}/.cargo" --locked
    cp "${HOME}/.cargo/bin/tailscale" "${BINARY_PATH}"
  fi
}

# ── Install via pre-built release binary ──────────────────────────────────────

install_from_release() {
  if ! command -v curl >/dev/null 2>&1; then
    error "curl not found. Install curl and re-run, or set BUILD=1 to build from source."
    return 1
  fi

  local tag="${TAILSCALE_RMCP_VERSION:-}"
  if [[ -z "${tag}" ]]; then
    info "Fetching latest release tag..."
    tag="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | \
           python3 -c "import sys,json; print(json.load(sys.stdin)['tag_name'])" 2>/dev/null)" || {
      warn "Could not determine latest release. Falling back to source build."
      install_from_cargo
      return
    }
  fi

  local os arch
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  arch="$(uname -m)"
  case "${arch}" in x86_64) arch="x86_64" ;; aarch64|arm64) arch="aarch64" ;; esac

  local asset="tailscale-${tag}-${arch}-${os}"
  local url="https://github.com/${REPO}/releases/download/${tag}/${asset}"

  info "Downloading ${asset} from GitHub Releases..."
  if ! curl -fsSL "${url}" -o "${BINARY_PATH}.tmp"; then
    warn "Release binary not found at ${url} — falling back to source build."
    rm -f "${BINARY_PATH}.tmp"
    install_from_cargo
    return
  fi

  chmod 755 "${BINARY_PATH}.tmp"
  mv "${BINARY_PATH}.tmp" "${BINARY_PATH}"
}

# ── Create data directory ──────────────────────────────────────────────────────

setup_data_dir() {
  info "Creating data directory ${DATA_DIR}/..."
  mkdir -p "${DATA_DIR}/logs"
  ok "Data directory: ${DATA_DIR}/ (created)"
  ok "Log directory:  ${DATA_DIR}/logs/ (created)"
}

# ── Write starter .env ─────────────────────────────────────────────────────────

write_env() {
  if [[ -f "${ENV_FILE}" ]]; then
    info "${ENV_FILE} already exists — skipping (not overwriting)"
    return
  fi

  info "Writing starter ${ENV_FILE}..."
  cat > "${ENV_FILE}" << 'EOF'
# tailscale-rmcp — Tailscale MCP Server
# Fill in the required values below, then run: tailscale doctor

# ── Required ─────────────────────────────────────────────────────────────────

# Tailscale API key — create at https://login.tailscale.com/admin/settings/keys
TAILSCALE_API_KEY=

# Tailnet: use "-" for personal accounts, "example.com" for org accounts
TAILSCALE_TAILNET=-

# MCP bearer token — generate with: openssl rand -hex 32
TAILSCALE_MCP_TOKEN=

# ── Optional ──────────────────────────────────────────────────────────────────

# Enable destructive operations (delete_device). Requires confirm=true in calls.
# WARNING: enabling this allows devices to be permanently deleted from your tailnet.
# TAILSCALE_ALLOW_DESTRUCTIVE=false

# Bind host and port (defaults: 0.0.0.0:7575)
# TAILSCALE_MCP_HOST=0.0.0.0
# TAILSCALE_MCP_PORT=7575

# Disable auth entirely (loopback-only safe)
# TAILSCALE_MCP_NO_AUTH=false
EOF
  chmod 600 "${ENV_FILE}"
  ok "Wrote ${ENV_FILE} (mode 600)"
}

# ── Post-install doctor check ─────────────────────────────────────────────────

post_install_doctor() {
  echo ""
  info "Running doctor check..."
  if "${BINARY_PATH}" doctor 2>/dev/null; then
    echo ""
    ok "Installation complete and verified."
  else
    echo ""
    warn "Installation complete but doctor found issues."
    warn "Fix the reported issues, then run: ${BINARY_NAME} serve"
  fi
}

# ── Main ──────────────────────────────────────────────────────────────────────

main() {
  printf '\n%b%s%b\n' "${C_BOLD}" "tailscale-rmcp installer" "${C_RESET}"
  printf '%s\n' "Tailscale MCP server — https://github.com/${REPO}"

  if [[ "${BINARY_NAME}" != "tailscale" ]]; then
    info "Installing as '${BINARY_NAME}' (alternative name — avoids shadowing the real Tailscale CLI)"
  fi

  # Pre-flight checks
  if ! preflight; then
    exit 1
  fi

  mkdir -p "${INSTALL_DIR}"

  # Conflict check — if it returns 1 and FORCE is unset, skip binary install
  local skip_binary=false
  if ! warn_binary_conflict; then
    if [[ "${FORCE:-false}" != "true" ]]; then
      skip_binary=true
    fi
  fi

  if [[ "${skip_binary}" == "false" ]]; then
    if [[ "${BUILD:-false}" == "true" ]]; then
      install_from_cargo
    else
      install_from_release
    fi

    if [[ -x "${BINARY_PATH}" ]]; then
      ok "Installed: ${BINARY_PATH}"
      # Ensure ~/.local/bin is in PATH hint
      if [[ ":${PATH}:" != *":${INSTALL_DIR}:"* ]]; then
        warn "${INSTALL_DIR} is not in your PATH."
        warn "Add this to your shell profile:"
        warn "  export PATH=\"\${HOME}/.local/bin:\${PATH}\""
      fi
    fi
  fi

  setup_data_dir
  write_env
  post_install_doctor

  printf '\n%b== Next steps ==%b\n' "${C_BOLD}" "${C_RESET}"
  printf '  1. Edit %s and set TAILSCALE_API_KEY and TAILSCALE_MCP_TOKEN\n' "${ENV_FILE}"
  printf '  2. Validate:       %s doctor\n' "${BINARY_NAME}"
  printf '  3. Start server:   %s serve\n' "${BINARY_NAME}"
  printf '  4. Connect Claude: add http://localhost:%s/mcp as an MCP server\n' "${MCP_PORT}"
  printf '     with header: Authorization: Bearer <your TAILSCALE_MCP_TOKEN>\n'
  printf '\n  Or deploy with Docker:\n'
  printf '     docker compose up -d\n'
  if [[ "${BINARY_NAME}" == "tailscale" ]]; then
    printf '\n%b  Binary conflict note:%b\n' "${C_YELLOW}" "${C_RESET}"
    printf '  If the real Tailscale CLI is installed, use BINARY_NAME=tailscale-mcp\n'
    printf '  to install tailscale-rmcp under a non-conflicting name:\n'
    printf '    BINARY_NAME=tailscale-mcp bash install.sh\n'
  fi
  printf '\n'
}

main "$@"
