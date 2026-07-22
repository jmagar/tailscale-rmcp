#!/bin/sh
# entrypoint.sh — Docker container entrypoint for tailscale-rmcp (Tailscale MCP server)
#
# Runs as root, validates the environment, fixes directory ownership, then
# drops privileges to UID 1000:1000 and exec's the service binary.
#
# Defense in numbers: validate every assumption before exec'ing the service.
# Fail fast with clear messages rather than starting in a broken state.
#
# The container binary matches the canonical CLI name.
set -e

DATA_DIR="${DATA_DIR:-/data}"
SERVICE_NAME="rtailscale"
BINARY="/usr/local/bin/${SERVICE_NAME}"

# ── 1. Binary exists and is executable ───────────────────────────────────────
if [ ! -x "${BINARY}" ]; then
    echo "FATAL: ${BINARY} is missing or not executable" >&2
    exit 1
fi

# ── 2. Required environment variables ────────────────────────────────────────
# Fail fast with a clear message rather than a cryptic runtime error.
missing_vars=""
for var in TAILSCALE_API_KEY; do
    # POSIX-safe indirect variable expansion
    eval "val=\${${var}:-}"
    if [ -z "${val}" ]; then
        missing_vars="${missing_vars} ${var}"
    fi
done
if [ -n "${missing_vars}" ]; then
    echo "FATAL: required environment variables not set:${missing_vars}" >&2
    echo "  Set them in your .env file or with docker run -e flags." >&2
    echo "  Example: -e TAILSCALE_API_KEY=tskey-api-..." >&2
    exit 1
fi

# ── 3. TAILSCALE_TAILNET default ──────────────────────────────────────────────
# '-' means personal account; this is a safe default, not a secret.
if [ -z "${TAILSCALE_TAILNET:-}" ]; then
    export TAILSCALE_TAILNET="-"
    echo "[entrypoint] TAILSCALE_TAILNET not set — defaulting to '-' (personal account)"
fi

# ── 4. Data directory setup ───────────────────────────────────────────────────
mkdir -p "${DATA_DIR}/logs"

# Fix ownership — volume may have been created by root or a different UID.
if ! chown -R 1000:1000 "${DATA_DIR}" 2>/dev/null; then
    echo "WARN: could not chown ${DATA_DIR} to 1000:1000 — permissions may be wrong" >&2
fi

# Verify the data dir is actually writable by UID 1000 before handing off.
if ! gosu 1000:1000 sh -c "touch '${DATA_DIR}/.write_test' 2>/dev/null && rm -f '${DATA_DIR}/.write_test'"; then
    echo "FATAL: ${DATA_DIR} is not writable by UID 1000" >&2
    echo "  Check the volume mount permissions." >&2
    exit 1
fi

# ── 5. Secure secret files ────────────────────────────────────────────────────
for f in "${DATA_DIR}/.env" "${DATA_DIR}/auth-jwt.pem" "${DATA_DIR}/auth.db"; do
    [ -f "${f}" ] && chmod 600 "${f}" 2>/dev/null || true
done
[ -f "${DATA_DIR}/config.toml" ] && chmod 640 "${DATA_DIR}/config.toml" 2>/dev/null || true

# ── 6. Destructive operations warning ────────────────────────────────────────
if [ "${TAILSCALE_ALLOW_DESTRUCTIVE:-false}" = "true" ] \
    || [ "${TAILSCALE_ALLOW_DESTRUCTIVE:-false}" = "1" ]; then
    echo "WARN: TAILSCALE_ALLOW_DESTRUCTIVE=true — destructive operations (delete_device) are enabled!" >&2
    echo "WARN: Devices can be permanently deleted from your tailnet through the MCP tool." >&2
fi

# ── 7. Log startup info (redact secrets) ─────────────────────────────────────
echo "[entrypoint] Starting ${SERVICE_NAME}"
echo "[entrypoint] Data dir:  ${DATA_DIR}"
echo "[entrypoint] Binary:    ${BINARY}"
echo "[entrypoint] User:      1000:1000"
echo "[entrypoint] Tailnet:   ${TAILSCALE_TAILNET}"
[ -n "${TAILSCALE_MCP_PORT:-}"  ] && echo "[entrypoint] MCP port:  ${TAILSCALE_MCP_PORT}"
[ -n "${TAILSCALE_MCP_HOST:-}"  ] && echo "[entrypoint] MCP host:  ${TAILSCALE_MCP_HOST}"
# NOTE: Do NOT log TAILSCALE_API_KEY or TAILSCALE_MCP_TOKEN — these are secrets.

# ── 8. Signal handling ────────────────────────────────────────────────────────
# Do NOT trap signals here. exec + gosu replaces this shell with the service
# binary, so signals go directly to the service process for graceful shutdown.

# ── 9. Drop privileges and exec ──────────────────────────────────────────────
# exec replaces this shell — PID 1 becomes the actual service binary.
exec gosu 1000:1000 "${BINARY}" "$@"
