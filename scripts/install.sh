#!/usr/bin/env bash
set -euo pipefail

REPO="${TAILSCALE_RMCP_REPO:-jmagar/rtailscale}"
INSTALL_DIR="${INSTALL_DIR:-${HOME}/.local/bin}"
VERSION="${TAILSCALE_RMCP_VERSION:-latest}"
RELEASE_BASE_URL="${TAILSCALE_RMCP_RELEASE_BASE_URL:-}"
BINARY_NAME="rtailscale"

usage() {
  cat <<'USAGE'
Install rtailscale from GitHub Releases.

Environment:
  INSTALL_DIR      Destination directory (default: ~/.local/bin)
  TAILSCALE_RMCP_VERSION Release tag such as v0.1.1 (default: latest)
  TAILSCALE_RMCP_REPO    GitHub repo owner/name (default: jmagar/rtailscale)
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

need() {
  command -v "$1" >/dev/null 2>&1 || {
    printf 'error: %s is required
' "$1" >&2
    exit 1
  }
}

target_asset() {
  local os arch
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  arch="$(uname -m)"

  case "${os}:${arch}" in
    linux:x86_64|linux:amd64)
      printf '%s-x86_64.tar.gz' "${BINARY_NAME}"
      ;;
    mingw*:x86_64|msys*:x86_64|cygwin*:x86_64)
      printf '%s-windows-x86_64.tar.gz' "${BINARY_NAME}"
      ;;
    *)
      printf 'error: unsupported platform %s/%s
' "${os}" "${arch}" >&2
      exit 1
      ;;
  esac
}

need curl
need install
need mktemp
need tar

asset="$(target_asset)"
tmpdir="$(mktemp -d)"
trap 'rm -rf "${tmpdir}"' EXIT

if [[ -n "${RELEASE_BASE_URL}" ]]; then
  url="${RELEASE_BASE_URL%/}/${VERSION}/${asset}"
elif [[ "${VERSION}" == "latest" ]]; then
  url="https://github.com/${REPO}/releases/latest/download/${asset}"
else
  url="https://github.com/${REPO}/releases/download/${VERSION}/${asset}"
fi

mkdir -p "${INSTALL_DIR}"
if [[ ! -w "${INSTALL_DIR}" ]]; then
  printf 'error: install dir is not writable: %s
' "${INSTALL_DIR}" >&2
  exit 1
fi

printf 'Downloading %s
' "${url}" >&2
curl -fsSL "${url}" -o "${tmpdir}/${asset}"
tar -xzf "${tmpdir}/${asset}" -C "${tmpdir}"

binary="${tmpdir}/${BINARY_NAME}"
if [[ ! -f "${binary}" && -f "${tmpdir}/${BINARY_NAME}.exe" ]]; then
  binary="${tmpdir}/${BINARY_NAME}.exe"
fi
if [[ ! -f "${binary}" ]]; then
  printf 'error: archive did not contain %s binary
' "${BINARY_NAME}" >&2
  exit 1
fi

install -m 755 "${binary}" "${INSTALL_DIR}/${BINARY_NAME}"
printf 'Installed %s to %s/%s
' "${BINARY_NAME}" "${INSTALL_DIR}" "${BINARY_NAME}"
printf 'Run: %s --version
' "${BINARY_NAME}"
