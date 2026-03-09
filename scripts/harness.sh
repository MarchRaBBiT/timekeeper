#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

BACKEND_BASE_URL="${BACKEND_BASE_URL:-http://localhost:3000}"
FRONTEND_BASE_URL="${FRONTEND_BASE_URL:-http://localhost:8080}"
ADMIN_USER="${E2E_ADMIN_USER:-admin}"
ADMIN_PASS="${E2E_ADMIN_PASS:-admin123}"
PROFILE="agent-fast"
CONTINUE_ON_FAIL=0
VERBOSE=0

declare -a REQUESTED_STAGES=()
declare -A STAGE_STATUS=()
declare -A STAGE_DURATION=()

usage() {
  cat <<'EOF'
Usage: bash scripts/harness.sh [profile|stage ...] [options]

Profiles
  agent-fast            doctor + backend-unit (default)
  smoke                 doctor + api-smoke + frontend-login
  full                  doctor + backend-unit + backend-integration + api-smoke + frontend-login + e2e

Stages
  doctor                Verify required commands, packages, and live URLs for selected stages
  backend-unit          Run backend library tests
  backend-integration   Run container-backed backend tests
  api-smoke             Run API smoke checks against a live backend
  frontend-login        Run a lightweight Playwright login flow against a live frontend
  e2e                   Run the Playwright smoke suite under e2e/run.mjs

Options
  --profile NAME        Use a named profile
  --stage NAME          Add an individual stage (repeatable)
  --backend-url URL     Backend base URL for live checks (default: http://localhost:3000)
  --frontend-url URL    Frontend base URL for live checks (default: http://localhost:8080)
  --admin-user USER     Admin username for smoke/E2E flows (default: admin)
  --admin-pass PASS     Admin password for smoke/E2E flows (default: admin123)
  --continue-on-fail    Run all requested stages and summarize failures at the end
  --verbose             Pass verbose output into delegated scripts where supported
  --list                Print the available profiles and stages
  --help                Show this help

Examples
  bash scripts/harness.sh
  bash scripts/harness.sh smoke --backend-url http://localhost:3000 --frontend-url http://localhost:8000
  bash scripts/harness.sh --stage doctor --stage api-smoke --backend-url http://localhost:3000
EOF
}

list_targets() {
  cat <<'EOF'
Profiles:
  agent-fast
  smoke
  full

Stages:
  doctor
  backend-unit
  backend-integration
  api-smoke
  frontend-login
  e2e
EOF
}

log_step() { printf '\n=== %s ===\n' "$1"; }
log_info() { printf '[INFO] %s\n' "$1"; }
log_ok() { printf '[OK] %s\n' "$1"; }
log_warn() { printf '[WARN] %s\n' "$1"; }
log_fail() { printf '[FAIL] %s\n' "$1" >&2; }

contains_stage() {
  local needle="$1"
  shift
  local item
  for item in "$@"; do
    if [[ "$item" == "$needle" ]]; then
      return 0
    fi
  done
  return 1
}

profile_stages() {
  case "$1" in
    agent-fast)
      printf '%s\n' doctor backend-unit
      ;;
    smoke)
      printf '%s\n' doctor api-smoke frontend-login
      ;;
    full)
      printf '%s\n' doctor backend-unit backend-integration api-smoke frontend-login e2e
      ;;
    *)
      log_fail "Unknown profile: $1"
      exit 1
      ;;
  esac
}

stage_description() {
  case "$1" in
    doctor) printf '%s' "environment and URL preflight" ;;
    backend-unit) printf '%s' "backend library tests" ;;
    backend-integration) printf '%s' "container-backed backend tests" ;;
    api-smoke) printf '%s' "live backend API smoke checks" ;;
    frontend-login) printf '%s' "Playwright login smoke against the frontend" ;;
    e2e) printf '%s' "Playwright end-to-end smoke suite" ;;
    *) printf '%s' "unknown stage" ;;
  esac
}

assert_known_stage() {
  case "$1" in
    doctor|backend-unit|backend-integration|api-smoke|frontend-login|e2e) ;;
    *)
      log_fail "Unknown stage: $1"
      exit 1
      ;;
  esac
}

have_cmd() {
  command -v "$1" >/dev/null 2>&1
}

have_python() {
  have_cmd python3 || have_cmd python
}

have_podman_compose() {
  if have_cmd podman && podman compose version >/dev/null 2>&1; then
    return 0
  fi
  have_cmd podman-compose
}

resolve_playwright() {
  node -e "require.resolve('playwright', { paths: [process.argv[1]] })" "${PROJECT_ROOT}/e2e" >/dev/null 2>&1
}

probe_url() {
  local url="$1"
  local status
  status="$(curl -ksS -o /dev/null -w '%{http_code}' --max-time 5 "$url" || true)"
  [[ "$status" != "000" && -n "$status" ]]
}

selected_stages=()

stage_needs_backend_url() {
  contains_stage "$1" api-smoke frontend-login e2e
}

stage_needs_frontend_url() {
  contains_stage "$1" frontend-login e2e
}

run_doctor() {
  local fail_count=0
  local need_live_backend=0
  local need_live_frontend=0
  local stage

  log_info "Selected stages: ${selected_stages[*]}"

  for stage in "${selected_stages[@]}"; do
    if stage_needs_backend_url "$stage"; then
      need_live_backend=1
    fi
    if stage_needs_frontend_url "$stage"; then
      need_live_frontend=1
    fi
  done

  if contains_stage backend-unit "${selected_stages[@]}" || contains_stage backend-integration "${selected_stages[@]}"; then
    if have_cmd cargo; then
      log_ok "cargo"
    else
      log_fail "cargo is required"
      fail_count=$((fail_count + 1))
    fi
  fi

  if contains_stage backend-integration "${selected_stages[@]}"; then
    if have_cmd podman; then
      log_ok "podman"
    else
      log_fail "podman is required for backend-integration"
      fail_count=$((fail_count + 1))
    fi
    if have_podman_compose; then
      log_ok "podman compose"
    else
      log_fail "podman compose or podman-compose is required for backend-integration"
      fail_count=$((fail_count + 1))
    fi
  fi

  if contains_stage api-smoke "${selected_stages[@]}"; then
    if have_cmd curl; then
      log_ok "curl"
    else
      log_fail "curl is required for api-smoke"
      fail_count=$((fail_count + 1))
    fi
    if have_python; then
      log_ok "python"
    else
      log_fail "python3 or python is required for api-smoke"
      fail_count=$((fail_count + 1))
    fi
  fi

  if contains_stage frontend-login "${selected_stages[@]}" || contains_stage e2e "${selected_stages[@]}"; then
    if have_cmd node; then
      log_ok "node"
    else
      log_fail "node is required for frontend-login/e2e"
      fail_count=$((fail_count + 1))
    fi
    if [[ -f "${PROJECT_ROOT}/e2e/package.json" ]]; then
      log_ok "e2e/package.json"
    else
      log_fail "e2e/package.json is missing"
      fail_count=$((fail_count + 1))
    fi
    if have_cmd node && resolve_playwright; then
      log_ok "playwright dependency (resolved via e2e)"
    else
      log_fail "playwright dependency could not be resolved from e2e/"
      fail_count=$((fail_count + 1))
    fi
  fi

  if [[ "$need_live_backend" -eq 1 ]]; then
    if probe_url "${BACKEND_BASE_URL}/api/docs"; then
      log_ok "backend reachable at ${BACKEND_BASE_URL}"
    else
      log_fail "backend is not reachable at ${BACKEND_BASE_URL} (expected /api/docs to respond)"
      fail_count=$((fail_count + 1))
    fi
  fi

  if [[ "$need_live_frontend" -eq 1 ]]; then
    if probe_url "${FRONTEND_BASE_URL}/login"; then
      log_ok "frontend reachable at ${FRONTEND_BASE_URL}"
    else
      log_fail "frontend is not reachable at ${FRONTEND_BASE_URL} (expected /login to respond)"
      fail_count=$((fail_count + 1))
    fi
  fi

  if [[ "$fail_count" -ne 0 ]]; then
    return 1
  fi
}

run_backend_unit() {
  (
    cd "${PROJECT_ROOT}/backend"
    cargo test --lib
  )
}

run_backend_integration() {
  (
    cd "${PROJECT_ROOT}"
    ./scripts/test_backend_integrated.sh
  )
}

run_api_smoke() {
  local args=(--base-url "${BACKEND_BASE_URL}" --admin-user "${ADMIN_USER}" --admin-pass "${ADMIN_PASS}")
  if [[ "${VERBOSE}" -eq 1 ]]; then
    args+=(--verbose)
  fi
  (
    cd "${PROJECT_ROOT}"
    bash ./scripts/test_backend.sh "${args[@]}"
  )
}

run_frontend_login() {
  (
    cd "${PROJECT_ROOT}"
    FRONTEND_BASE_URL="${FRONTEND_BASE_URL}" \
    NODE_PATH="${PROJECT_ROOT}/e2e/node_modules${NODE_PATH:+:${NODE_PATH}}" \
    node ./scripts/test_frontend_login.mjs
  )
}

run_e2e() {
  (
    cd "${PROJECT_ROOT}/e2e"
    FRONTEND_BASE_URL="${FRONTEND_BASE_URL}" \
    E2E_ADMIN_USER="${ADMIN_USER}" \
    E2E_ADMIN_PASS="${ADMIN_PASS}" \
    node ./run.mjs
  )
}

execute_stage() {
  local stage="$1"
  local stage_func
  local started_at
  local ended_at
  local duration

  log_step "${stage}: $(stage_description "$stage")"
  started_at="$(date +%s)"
  stage_func="run_${stage//-/_}"

  if "${stage_func}"; then
    STAGE_STATUS["$stage"]="passed"
  else
    STAGE_STATUS["$stage"]="failed"
    ended_at="$(date +%s)"
    duration=$((ended_at - started_at))
    STAGE_DURATION["$stage"]="${duration}s"
    if [[ "${CONTINUE_ON_FAIL}" -ne 1 ]]; then
      print_summary
      exit 1
    fi
    return 1
  fi

  ended_at="$(date +%s)"
  duration=$((ended_at - started_at))
  STAGE_DURATION["$stage"]="${duration}s"
  log_ok "${stage} completed in ${duration}s"
  return 0
}

print_summary() {
  local stage
  printf '\nHarness summary\n'
  printf '%-22s %-10s %s\n' "stage" "status" "duration"
  printf '%-22s %-10s %s\n' "----------------------" "----------" "--------"
  for stage in "${selected_stages[@]}"; do
    printf '%-22s %-10s %s\n' \
      "$stage" \
      "${STAGE_STATUS[$stage]:-skipped}" \
      "${STAGE_DURATION[$stage]:--}"
  done
}

if [[ $# -eq 0 ]]; then
  while IFS= read -r stage; do
    selected_stages+=("$stage")
  done < <(profile_stages "$PROFILE")
else
  while [[ $# -gt 0 ]]; do
    case "$1" in
      agent-fast|smoke|full)
        PROFILE="$1"
        shift
        ;;
      doctor|backend-unit|backend-integration|api-smoke|frontend-login|e2e)
        REQUESTED_STAGES+=("$1")
        shift
        ;;
      --profile)
        PROFILE="$2"
        shift 2
        ;;
      --stage)
        assert_known_stage "$2"
        REQUESTED_STAGES+=("$2")
        shift 2
        ;;
      --backend-url)
        BACKEND_BASE_URL="$2"
        shift 2
        ;;
      --frontend-url)
        FRONTEND_BASE_URL="$2"
        shift 2
        ;;
      --admin-user)
        ADMIN_USER="$2"
        shift 2
        ;;
      --admin-pass)
        ADMIN_PASS="$2"
        shift 2
        ;;
      --continue-on-fail)
        CONTINUE_ON_FAIL=1
        shift
        ;;
      --verbose)
        VERBOSE=1
        shift
        ;;
      --list)
        list_targets
        exit 0
        ;;
      --help|-h)
        usage
        exit 0
        ;;
      *)
        log_fail "Unknown argument: $1"
        usage
        exit 1
        ;;
    esac
  done

  if [[ "${#REQUESTED_STAGES[@]}" -gt 0 ]]; then
    selected_stages=("${REQUESTED_STAGES[@]}")
  else
    while IFS= read -r stage; do
      selected_stages+=("$stage")
    done < <(profile_stages "$PROFILE")
  fi
fi

for stage in "${selected_stages[@]}"; do
  assert_known_stage "$stage"
done

log_info "Harness profile: ${PROFILE}"
log_info "Backend URL: ${BACKEND_BASE_URL}"
log_info "Frontend URL: ${FRONTEND_BASE_URL}"

for stage in "${selected_stages[@]}"; do
  execute_stage "$stage" || true
done

print_summary

for stage in "${selected_stages[@]}"; do
  if [[ "${STAGE_STATUS[$stage]:-}" == "failed" ]]; then
    exit 1
  fi
done
