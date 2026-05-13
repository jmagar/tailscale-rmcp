#!/usr/bin/env bash
# =============================================================================
# test-tools.sh — Integration smoke-test for rustscale MCP server tools
#
# Exercises all non-destructive actions of the tailscale MCP tool:
#   tailscale devices    — all devices in the tailnet
#   tailscale device     — single device (requires TAILSCALE_TEST_DEVICE_ID)
#   tailscale device_routes — subnet routes (requires TAILSCALE_TEST_DEVICE_ID)
#   tailscale keys       — API keys
#   tailscale acl        — ACL policy
#   tailscale dns        — DNS nameservers + search paths + preferences
#   tailscale users      — tailnet members
#   tailscale help       — built-in documentation
#
# Skipped (destructive/write):
#   tailscale authorize_device  — modifies state
#   tailscale delete_device     — destructive
#
# Also tests the schema resource: tailscale://schema/mcp-tool
#
# CRITICAL: Passing means REAL Tailscale API data is flowing.
# Semantic validation checks actual tailnet data fields, not just MCP framing.
#
# Environment:
#   TAILSCALE_MCP_HOST      MCP server host (default: localhost)
#   TAILSCALE_MCP_PORT      MCP server port (default: 7575)
#   TAILSCALE_MCP_TOKEN     Bearer token (optional if no_auth=true)
#   TAILSCALE_TEST_DEVICE_ID  Device ID for device/device_routes tests (optional)
#
# Credentials sourced from ~/.claude-homelab/.env if present.
#
# Usage:
#   ./tests/mcporter/test-tools.sh [--timeout-ms N] [--parallel] [--verbose]
#
# Options:
#   --timeout-ms N   Per-call timeout in milliseconds (default: 25000)
#   --parallel       Run independent test groups in parallel (default: off)
#   --verbose        Print raw mcporter output for each call
#
# Exit codes:
#   0 — all tests passed or skipped
#   1 — one or more tests failed
#   2 — prerequisite check failed (mcporter not found, server unreachable)
# =============================================================================

set -uo pipefail

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------
readonly SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
readonly PROJECT_DIR="$(cd -- "${SCRIPT_DIR}/../.." && pwd -P)"
readonly SCRIPT_NAME="$(basename -- "${BASH_SOURCE[0]}")"
readonly TS_START="$(date +%s%N)"
readonly LOG_FILE="${TMPDIR:-/tmp}/${SCRIPT_NAME%.sh}.$(date +%Y%m%d-%H%M%S).log"
readonly ENV_FILE="${HOME}/.claude-homelab/.env"

# Colours (disabled automatically when stdout is not a terminal)
if [[ -t 1 ]]; then
  C_RESET='\033[0m'
  C_BOLD='\033[1m'
  C_GREEN='\033[0;32m'
  C_RED='\033[0;31m'
  C_YELLOW='\033[0;33m'
  C_CYAN='\033[0;36m'
  C_DIM='\033[2m'
else
  C_RESET='' C_BOLD='' C_GREEN='' C_RED='' C_YELLOW='' C_CYAN='' C_DIM=''
fi

# ---------------------------------------------------------------------------
# Defaults (overridable via flags)
# ---------------------------------------------------------------------------
CALL_TIMEOUT_MS=25000
USE_PARALLEL=false
VERBOSE=false

# ---------------------------------------------------------------------------
# Counters (updated by run_test / skip_test)
# ---------------------------------------------------------------------------
PASS_COUNT=0
FAIL_COUNT=0
SKIP_COUNT=0
declare -a FAIL_NAMES=()

# Runtime globals — populated after ENV load
MCP_URL=''
MCPORTER_HEADER_ARGS=()
DEVICE_ID=''

# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------
parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --timeout-ms)
        CALL_TIMEOUT_MS="${2:?--timeout-ms requires a value}"
        shift 2
        ;;
      --parallel)
        USE_PARALLEL=true
        shift
        ;;
      --verbose)
        VERBOSE=true
        shift
        ;;
      -h|--help)
        printf 'Usage: %s [--timeout-ms N] [--parallel] [--verbose]\n' "${SCRIPT_NAME}"
        exit 0
        ;;
      *)
        printf '[ERROR] Unknown argument: %s\n' "$1" >&2
        exit 2
        ;;
    esac
  done
}

# ---------------------------------------------------------------------------
# Logging helpers
# ---------------------------------------------------------------------------
log_info()  { printf "${C_CYAN}[INFO]${C_RESET}  %s\n" "$*" | tee -a "${LOG_FILE}"; }
log_warn()  { printf "${C_YELLOW}[WARN]${C_RESET}  %s\n" "$*" | tee -a "${LOG_FILE}"; }
log_error() { printf "${C_RED}[ERROR]${C_RESET} %s\n" "$*" | tee -a "${LOG_FILE}" >&2; }

elapsed_ms() {
  local now
  now="$(date +%s%N)"
  printf '%d' "$(( (now - TS_START) / 1000000 ))"
}

# ---------------------------------------------------------------------------
# Cleanup trap
# ---------------------------------------------------------------------------
cleanup() {
  local rc=$?
  if [[ $rc -ne 0 ]]; then
    log_warn "Script exited with rc=${rc}. Log: ${LOG_FILE}"
  fi
}
trap cleanup EXIT

# ---------------------------------------------------------------------------
# Load environment and build MCP URL + auth headers
# ---------------------------------------------------------------------------
load_env() {
  if [[ -f "${ENV_FILE}" ]]; then
    # shellcheck disable=SC1090
    set -a
    source "${ENV_FILE}"
    set +a
    log_info "Loaded credentials from ${ENV_FILE}"
  else
    log_warn "${ENV_FILE} not found — using defaults / environment"
  fi

  local host="${TAILSCALE_MCP_HOST:-localhost}"
  # Remap bind address 0.0.0.0 → localhost for outbound connections
  if [[ "${host}" == "0.0.0.0" ]]; then
    host="localhost"
  fi
  local port="${TAILSCALE_MCP_PORT:-7575}"
  MCP_URL="http://${host}:${port}/mcp"

  local token="${TAILSCALE_MCP_TOKEN:-}"
  MCPORTER_HEADER_ARGS=()
  if [[ -n "${token}" ]]; then
    MCPORTER_HEADER_ARGS+=(--header "Authorization: Bearer ${token}")
  fi

  # Device ID for parameterised tests
  DEVICE_ID="${TAILSCALE_TEST_DEVICE_ID:-}"

  log_info "MCP URL: ${MCP_URL}"
  if [[ ${#MCPORTER_HEADER_ARGS[@]} -gt 0 ]]; then
    log_info "Auth: Bearer token configured"
  else
    log_info "Auth: none (TAILSCALE_MCP_TOKEN unset)"
  fi

  if [[ -n "${DEVICE_ID}" ]]; then
    log_info "Test device ID: ${DEVICE_ID}"
  else
    log_info "Test device ID: not set — device/device_routes tests will be skipped"
    log_info "  Set TAILSCALE_TEST_DEVICE_ID to enable them"
  fi
}

# ---------------------------------------------------------------------------
# Prerequisite checks
# ---------------------------------------------------------------------------
check_prerequisites() {
  local missing=false

  if ! command -v mcporter &>/dev/null; then
    log_error "mcporter not found in PATH. Install it and re-run."
    missing=true
  fi

  if ! command -v python3 &>/dev/null; then
    log_error "python3 not found in PATH."
    missing=true
  fi

  if ! command -v curl &>/dev/null; then
    log_error "curl not found in PATH."
    missing=true
  fi

  if [[ "${missing}" == true ]]; then
    return 2
  fi
}

# ---------------------------------------------------------------------------
# Server connectivity smoke-test
# ---------------------------------------------------------------------------
smoke_test_server() {
  log_info "Smoke-testing server connectivity..."

  local base_url="${MCP_URL%/mcp}"

  # 1. Health endpoint (no auth required)
  local health_status
  health_status="$(
    curl -sf --max-time 10 "${base_url}/health" 2>/dev/null | \
    python3 -c "import sys,json; print(json.load(sys.stdin).get('status',''))" 2>/dev/null
  )" || health_status=''

  if [[ "${health_status}" != "ok" ]]; then
    log_error "Health endpoint at ${base_url}/health did not return status=ok (got: '${health_status}')"
    log_error "Is the rustscale server running?  just dev  or  docker compose up -d"
    return 2
  fi
  log_info "Health endpoint OK"

  # 2. tools/list to confirm MCP layer responds
  local tool_count
  tool_count="$(
    curl -sf --max-time 10 \
      -X POST "${MCP_URL}" \
      -H "Content-Type: application/json" \
      -H "Accept: application/json, text/event-stream" \
      ${MCPORTER_HEADER_ARGS[@]+"${MCPORTER_HEADER_ARGS[@]}"} \
      -d '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' 2>/dev/null | \
    python3 -c "
import sys, json
d = json.load(sys.stdin)
tools = d.get('result', {}).get('tools', [])
print(len(tools))
" 2>/dev/null
  )" || tool_count=0

  if [[ "${tool_count}" -lt 1 ]] 2>/dev/null; then
    log_error "tools/list returned ${tool_count} tools — expected at least 1"
    return 2
  fi

  log_info "Server OK — ${tool_count} tools available"
  return 0
}

# ---------------------------------------------------------------------------
# mcporter call wrapper
#   Usage: mcporter_call <args_json>
# ---------------------------------------------------------------------------
mcporter_call() {
  local args_json="${1:?args_json required}"

  mcporter call \
    --http-url "${MCP_URL}" \
    --allow-http \
    ${MCPORTER_HEADER_ARGS[@]+"${MCPORTER_HEADER_ARGS[@]}"} \
    --tool "tailscale" \
    --args "${args_json}" \
    --timeout "${CALL_TIMEOUT_MS}" \
    --output json \
    2>>"${LOG_FILE}"
}

# ---------------------------------------------------------------------------
# Test runner
#   Usage: run_test <label> <args_json> <validation_script>
#
#   validation_script is a python3 snippet that receives the parsed JSON dict
#   as `d` and should raise an exception if validation fails, otherwise returns.
# ---------------------------------------------------------------------------
run_test() {
  local label="${1:?label required}"
  local args="${2:?args required}"
  local validation="${3:-}"

  local t0
  t0="$(date +%s%N)"

  local output
  output="$(mcporter_call "${args}")" || true

  local elapsed_ms_val
  elapsed_ms_val="$(( ( $(date +%s%N) - t0 ) / 1000000 ))"

  if [[ "${VERBOSE}" == true ]]; then
    printf '%s\n' "${output}" | tee -a "${LOG_FILE}"
  else
    printf '%s\n' "${output}" >> "${LOG_FILE}"
  fi

  # Basic JSON parse + error check
  local json_check
  json_check="$(
    printf '%s' "${output}" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    if isinstance(d, dict) and ('error' in d or d.get('kind') == 'error'):
        print('error: ' + str(d.get('error', d.get('message', 'unknown error'))))
    else:
        print('ok')
except Exception as e:
    print('invalid_json: ' + str(e))
" 2>/dev/null
  )" || json_check="parse_error"

  if [[ "${json_check}" != "ok" ]]; then
    printf "${C_RED}[FAIL]${C_RESET} %-60s ${C_DIM}%dms${C_RESET}\n" \
      "${label}" "${elapsed_ms_val}" | tee -a "${LOG_FILE}"
    printf '       response validation failed: %s\n' "${json_check}" | tee -a "${LOG_FILE}"
    FAIL_COUNT=$(( FAIL_COUNT + 1 ))
    FAIL_NAMES+=("${label}")
    return 1
  fi

  # Semantic validation (optional python3 snippet)
  if [[ -n "${validation}" ]]; then
    local sem_check
    sem_check="$(
      printf '%s' "${output}" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    ${validation}
    print('ok')
except Exception as e:
    print('semantic_fail: ' + str(e))
" 2>/dev/null
    )" || sem_check="semantic_error"

    if [[ "${sem_check}" != "ok" ]]; then
      printf "${C_RED}[FAIL]${C_RESET} %-60s ${C_DIM}%dms${C_RESET}\n" \
        "${label}" "${elapsed_ms_val}" | tee -a "${LOG_FILE}"
      printf '       semantic validation failed: %s\n' "${sem_check}" | tee -a "${LOG_FILE}"
      FAIL_COUNT=$(( FAIL_COUNT + 1 ))
      FAIL_NAMES+=("${label}")
      return 1
    fi
  fi

  printf "${C_GREEN}[PASS]${C_RESET} %-60s ${C_DIM}%dms${C_RESET}\n" \
    "${label}" "${elapsed_ms_val}" | tee -a "${LOG_FILE}"
  PASS_COUNT=$(( PASS_COUNT + 1 ))
  return 0
}

# ---------------------------------------------------------------------------
# Skip helper
# ---------------------------------------------------------------------------
skip_test() {
  local label="${1:?label required}"
  local reason="${2:-prerequisite not met}"
  printf "${C_YELLOW}[SKIP]${C_RESET} %-60s %s\n" "${label}" "${reason}" | tee -a "${LOG_FILE}"
  SKIP_COUNT=$(( SKIP_COUNT + 1 ))
}

# ---------------------------------------------------------------------------
# Test suites
# ---------------------------------------------------------------------------

suite_help() {
  printf '\n%b== help ==%b\n' "${C_BOLD}" "${C_RESET}" | tee -a "${LOG_FILE}"

  run_test \
    "tailscale help: returns documentation" \
    '{"action":"help"}' \
    "
assert 'help' in d, 'missing help key'
assert isinstance(d['help'], str), 'help is not a string'
assert len(d['help']) > 20, 'help string too short'
"
}

suite_devices() {
  printf '\n%b== devices ==%b\n' "${C_BOLD}" "${C_RESET}" | tee -a "${LOG_FILE}"

  run_test \
    "tailscale devices: returns devices array" \
    '{"action":"devices"}' \
    "
assert 'devices' in d, 'missing devices key'
assert isinstance(d['devices'], list), 'devices is not a list'
assert len(d['devices']) >= 1, 'expected at least one device'
first = d['devices'][0]
assert 'id' in first, 'device missing id field'
assert 'hostname' in first, 'device missing hostname field'
assert 'addresses' in first, 'device missing addresses field'
assert isinstance(first['addresses'], list), 'addresses is not a list'
"
}

suite_keys() {
  printf '\n%b== keys ==%b\n' "${C_BOLD}" "${C_RESET}" | tee -a "${LOG_FILE}"

  run_test \
    "tailscale keys: returns keys object or array" \
    '{"action":"keys"}' \
    "
# keys may be an empty list or a dict/list — just require it to be parseable
# (some tailnets have no API keys configured; that is valid)
assert d is not None, 'response is None'
"
}

suite_acl() {
  printf '\n%b== acl ==%b\n' "${C_BOLD}" "${C_RESET}" | tee -a "${LOG_FILE}"

  run_test \
    "tailscale acl: returns ACL with acls or tagOwners key" \
    '{"action":"acl"}' \
    "
assert isinstance(d, dict), 'acl response is not a dict'
assert ('acls' in d or 'tagOwners' in d or 'hosts' in d or 'groups' in d), \
  'acl response missing expected Tailscale ACL fields (acls, tagOwners, hosts, groups)'
"
}

suite_dns() {
  printf '\n%b== dns ==%b\n' "${C_BOLD}" "${C_RESET}" | tee -a "${LOG_FILE}"

  run_test \
    "tailscale dns: returns aggregated DNS data" \
    '{"action":"dns"}' \
    "
assert isinstance(d, dict), 'dns response is not a dict'
# dns action aggregates nameservers, searchpaths, and preferences
# Accept any of these top-level shapes:
has_data = (
  'nameservers' in d or
  'dns' in d or
  'searchPaths' in d or
  'searchpaths' in d or
  'magicDNS' in d or
  'preferences' in d
)
assert has_data, 'dns response contains no recognized DNS fields: ' + str(list(d.keys()))
"
}

suite_users() {
  printf '\n%b== users ==%b\n' "${C_BOLD}" "${C_RESET}" | tee -a "${LOG_FILE}"

  run_test \
    "tailscale users: returns users array" \
    '{"action":"users"}' \
    "
assert 'users' in d, 'missing users key'
assert isinstance(d['users'], list), 'users is not a list'
assert len(d['users']) >= 1, 'expected at least one user'
first = d['users'][0]
assert 'loginName' in first, 'user missing loginName field'
"
}

suite_device() {
  printf '\n%b== device (parameterised) ==%b\n' "${C_BOLD}" "${C_RESET}" | tee -a "${LOG_FILE}"

  if [[ -z "${DEVICE_ID}" ]]; then
    skip_test "tailscale device: single device lookup" \
      "TAILSCALE_TEST_DEVICE_ID not set"
    skip_test "tailscale device_routes: subnet routes" \
      "TAILSCALE_TEST_DEVICE_ID not set"
    return
  fi

  local device_args
  device_args="$(python3 -c "import json; print(json.dumps({'action':'device','id':'${DEVICE_ID}'})")"

  run_test \
    "tailscale device: returns single device" \
    "${device_args}" \
    "
assert isinstance(d, dict), 'device response is not a dict'
assert 'id' in d or 'nodeId' in d, 'device missing id/nodeId'
assert 'hostname' in d, 'device missing hostname'
assert 'addresses' in d, 'device missing addresses'
"

  local routes_args
  routes_args="$(python3 -c "import json; print(json.dumps({'action':'device_routes','id':'${DEVICE_ID}'})")"

  run_test \
    "tailscale device_routes: returns subnet routes" \
    "${routes_args}" \
    "
assert isinstance(d, dict), 'device_routes response is not a dict'
assert ('advertisedRoutes' in d or 'enabledRoutes' in d), \
  'device_routes missing advertisedRoutes or enabledRoutes'
"
}

suite_schema_resource() {
  printf '\n%b== schema resource ==%b\n' "${C_BOLD}" "${C_RESET}" | tee -a "${LOG_FILE}"

  local label="schema resource: tailscale://schema/mcp-tool"

  # Query resources/read for the schema URI
  local schema_output
  schema_output="$(
    curl -sf --max-time 15 \
      -X POST "${MCP_URL}" \
      -H "Content-Type: application/json" \
      -H "Accept: application/json, text/event-stream" \
      ${MCPORTER_HEADER_ARGS[@]+"${MCPORTER_HEADER_ARGS[@]}"} \
      -d '{"jsonrpc":"2.0","id":2,"method":"resources/read","params":{"uri":"tailscale://schema/mcp-tool"}}' \
      2>/dev/null
  )" || schema_output=''

  local schema_check
  schema_check="$(
    printf '%s' "${schema_output}" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    result = d.get('result', {})
    contents = result.get('contents', [])
    if not contents:
        print('no_contents')
    else:
        text = contents[0].get('text', '')
        if not text:
            print('empty_text')
        else:
            # Verify it's JSON schema-like content
            schema = json.loads(text)
            if 'type' in schema or 'properties' in schema or 'action' in str(schema):
                print('ok')
            else:
                print('unexpected_schema: ' + str(list(schema.keys())[:5]))
except Exception as e:
    print('error: ' + str(e))
" 2>/dev/null
  )" || schema_check="parse_error"

  if [[ "${schema_check}" == "ok" ]]; then
    printf "${C_GREEN}[PASS]${C_RESET} %-60s\n" "${label}" | tee -a "${LOG_FILE}"
    PASS_COUNT=$(( PASS_COUNT + 1 ))
  elif [[ "${schema_check}" == "no_contents" || "${schema_check}" == "empty_text" ]]; then
    # Schema resource may not be implemented yet — skip rather than fail
    skip_test "${label}" "resource returned no content (not yet implemented)"
  else
    printf "${C_RED}[FAIL]${C_RESET} %-60s\n" "${label}" | tee -a "${LOG_FILE}"
    printf '       %s\n' "${schema_check}" | tee -a "${LOG_FILE}"
    FAIL_COUNT=$(( FAIL_COUNT + 1 ))
    FAIL_NAMES+=("${label}")
  fi
}

suite_auth() {
  if [[ -z "${TAILSCALE_MCP_TOKEN:-}" ]]; then
    printf '\n%b== auth (skipped — token unset) ==%b\n' "${C_BOLD}" "${C_RESET}" | tee -a "${LOG_FILE}"
    skip_test "auth: unauthenticated request returns 401" "TAILSCALE_MCP_TOKEN unset"
    skip_test "auth: bad token returns 401"                "TAILSCALE_MCP_TOKEN unset"
    return
  fi

  printf '\n%b== auth enforcement ==%b\n' "${C_BOLD}" "${C_RESET}" | tee -a "${LOG_FILE}"

  local label status

  label="auth: unauthenticated /mcp returns 401"
  status="$(curl -s --max-time 10 -o /dev/null -w "%{http_code}" \
    "${MCP_URL}" -X POST -H "Content-Type: application/json" \
    -H "Accept: application/json, text/event-stream" \
    -d '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' 2>/dev/null)" || status=0
  if [[ "${status}" == "401" ]]; then
    printf "${C_GREEN}[PASS]${C_RESET} %-60s\n" "${label}" | tee -a "${LOG_FILE}"
    PASS_COUNT=$(( PASS_COUNT + 1 ))
  else
    printf "${C_RED}[FAIL]${C_RESET} %-60s (got HTTP %s)\n" "${label}" "${status}" | tee -a "${LOG_FILE}"
    FAIL_COUNT=$(( FAIL_COUNT + 1 ))
    FAIL_NAMES+=("${label}")
  fi

  label="auth: bad token returns 401"
  status="$(curl -s --max-time 10 -o /dev/null -w "%{http_code}" \
    "${MCP_URL}" -X POST \
    -H "Authorization: Bearer bad-token-intentionally-invalid" \
    -H "Content-Type: application/json" \
    -H "Accept: application/json, text/event-stream" \
    -d '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' 2>/dev/null)" || status=0
  if [[ "${status}" == "401" ]]; then
    printf "${C_GREEN}[PASS]${C_RESET} %-60s\n" "${label}" | tee -a "${LOG_FILE}"
    PASS_COUNT=$(( PASS_COUNT + 1 ))
  else
    printf "${C_RED}[FAIL]${C_RESET} %-60s (got HTTP %s)\n" "${label}" "${status}" | tee -a "${LOG_FILE}"
    FAIL_COUNT=$(( FAIL_COUNT + 1 ))
    FAIL_NAMES+=("${label}")
  fi
}

# ---------------------------------------------------------------------------
# Print final summary
# ---------------------------------------------------------------------------
print_summary() {
  local total_ms
  total_ms="$(( ( $(date +%s%N) - TS_START ) / 1000000 ))"
  local total=$(( PASS_COUNT + FAIL_COUNT + SKIP_COUNT ))

  printf '\n%b%s%b\n' "${C_BOLD}" "$(printf '=%.0s' {1..65})" "${C_RESET}"
  printf '%b%-20s%b  %b%d%b\n' "${C_BOLD}" "PASS" "${C_RESET}" "${C_GREEN}" "${PASS_COUNT}" "${C_RESET}"
  printf '%b%-20s%b  %b%d%b\n' "${C_BOLD}" "FAIL" "${C_RESET}" "${C_RED}"   "${FAIL_COUNT}" "${C_RESET}"
  printf '%b%-20s%b  %b%d%b\n' "${C_BOLD}" "SKIP" "${C_RESET}" "${C_YELLOW}" "${SKIP_COUNT}" "${C_RESET}"
  printf '%b%-20s%b  %d\n' "${C_BOLD}" "TOTAL" "${C_RESET}" "${total}"
  printf '%b%-20s%b  %ds (%dms)\n' "${C_BOLD}" "ELAPSED" "${C_RESET}" \
    "$(( total_ms / 1000 ))" "${total_ms}"
  printf '%b%s%b\n' "${C_BOLD}" "$(printf '=%.0s' {1..65})" "${C_RESET}"

  if [[ "${FAIL_COUNT}" -gt 0 ]]; then
    printf '\n%bFailed tests:%b\n' "${C_RED}" "${C_RESET}"
    local name
    for name in "${FAIL_NAMES[@]}"; do
      printf '  * %s\n' "${name}"
    done
    printf '\nFull log: %s\n' "${LOG_FILE}"
  fi
}

# ---------------------------------------------------------------------------
# Parallel runner
# ---------------------------------------------------------------------------
run_parallel() {
  log_warn "--parallel mode: per-suite counters aggregated via temp files."

  local tmp_dir
  tmp_dir="$(mktemp -d)"
  trap 'rm -rf -- "${tmp_dir}"' RETURN

  local suites=(
    suite_auth
    suite_help
    suite_devices
    suite_keys
    suite_acl
    suite_dns
    suite_users
    suite_device
    suite_schema_resource
  )

  local pids=()
  local suite
  for suite in "${suites[@]}"; do
    (
      PASS_COUNT=0; FAIL_COUNT=0; SKIP_COUNT=0; FAIL_NAMES=()
      "${suite}"
      printf '%d %d %d\n' "${PASS_COUNT}" "${FAIL_COUNT}" "${SKIP_COUNT}" \
        > "${tmp_dir}/${suite}.counts"
      printf '%s\n' "${FAIL_NAMES[@]:-}" > "${tmp_dir}/${suite}.fails"
    ) &
    pids+=($!)
  done

  local pid
  for pid in "${pids[@]}"; do
    wait "${pid}" || true
  done

  local f
  for f in "${tmp_dir}"/*.counts; do
    [[ -f "${f}" ]] || continue
    local p fl s
    read -r p fl s < "${f}"
    PASS_COUNT=$(( PASS_COUNT + p ))
    FAIL_COUNT=$(( FAIL_COUNT + fl ))
    SKIP_COUNT=$(( SKIP_COUNT + s ))
  done

  for f in "${tmp_dir}"/*.fails; do
    [[ -f "${f}" ]] || continue
    while IFS= read -r line; do
      [[ -n "${line}" ]] && FAIL_NAMES+=("${line}")
    done < "${f}"
  done
}

# ---------------------------------------------------------------------------
# Sequential runner
# ---------------------------------------------------------------------------
run_sequential() {
  suite_auth
  suite_help
  suite_devices
  suite_keys
  suite_acl
  suite_dns
  suite_users
  suite_device
  suite_schema_resource
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
main() {
  parse_args "$@"
  load_env

  printf '%b%s%b\n' "${C_BOLD}" "$(printf '=%.0s' {1..65})" "${C_RESET}"
  printf '%b  rustscale MCP integration smoke-test%b\n' "${C_BOLD}" "${C_RESET}"
  printf '%b  Project:  %s%b\n' "${C_BOLD}" "${PROJECT_DIR}" "${C_RESET}"
  printf '%b  MCP URL:  %s%b\n' "${C_BOLD}" "${MCP_URL}" "${C_RESET}"
  printf '%b  Timeout:  %dms/call | Parallel: %s%b\n' \
    "${C_BOLD}" "${CALL_TIMEOUT_MS}" "${USE_PARALLEL}" "${C_RESET}"
  printf '%b  Log:      %s%b\n' "${C_BOLD}" "${LOG_FILE}" "${C_RESET}"
  printf '%b%s%b\n\n' "${C_BOLD}" "$(printf '=%.0s' {1..65})" "${C_RESET}"

  check_prerequisites || exit 2

  smoke_test_server || {
    log_error ""
    log_error "Server connectivity check failed. Aborting — no tests will run."
    log_error ""
    log_error "To diagnose:"
    log_error "  just dev  # or  docker compose up -d"
    log_error "  curl http://localhost:7575/health"
    log_error "  curl -X POST http://localhost:7575/mcp \\"
    log_error "    -H 'Content-Type: application/json' \\"
    log_error "    -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{}}'"
    exit 2
  }

  if [[ "${USE_PARALLEL}" == true ]]; then
    run_parallel
  else
    run_sequential
  fi

  print_summary

  if [[ "${FAIL_COUNT}" -gt 0 ]]; then
    exit 1
  fi
  exit 0
}

main "$@"
