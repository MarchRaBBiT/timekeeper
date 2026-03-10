#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BACKEND_BASE_URL="${BACKEND_BASE_URL:-http://localhost:3000}"
FRONTEND_BASE_URL="${FRONTEND_BASE_URL:-http://localhost:8080}"

log() {
  printf '[harness] %s\n' "$*"
}

die() {
  printf '[harness][FAIL] %s\n' "$*" >&2
  exit 1
}

usage() {
  cat <<'EOF'
Usage:
  bash scripts/harness.sh --list
  bash scripts/harness.sh doctor
  bash scripts/harness.sh backend-unit
  bash scripts/harness.sh backend-integration
  bash scripts/harness.sh api-smoke
  bash scripts/harness.sh frontend-login
  bash scripts/harness.sh smoke
  bash scripts/harness.sh full

Environment:
  BACKEND_BASE_URL   default: http://localhost:3000
  FRONTEND_BASE_URL  default: http://localhost:8080
EOF
}

require_cmd() {
  local cmd="$1"
  command -v "$cmd" >/dev/null 2>&1 || die "missing command: $cmd"
}

check_url() {
  local url="$1"
  curl -fsS --max-time 5 "$url" >/dev/null
}

run_doctor() {
  log "stage=doctor"
  require_cmd bash
  require_cmd cargo
  require_cmd node
  require_cmd curl
  if command -v python3 >/dev/null 2>&1; then
    :
  elif command -v python >/dev/null 2>&1; then
    :
  else
    die "missing command: python3 or python"
  fi
  log "BACKEND_BASE_URL=$BACKEND_BASE_URL"
  log "FRONTEND_BASE_URL=$FRONTEND_BASE_URL"
}

run_backend_unit() {
  log "stage=backend-unit"
  (cd "$ROOT_DIR" && cargo test -p timekeeper-backend --lib)
}

run_backend_integration() {
  log "stage=backend-integration"
  (cd "$ROOT_DIR" && cargo test -p timekeeper-backend --tests)
}

run_api_smoke() {
  log "stage=api-smoke"
  check_url "$BACKEND_BASE_URL/health" || check_url "$BACKEND_BASE_URL"
  (cd "$ROOT_DIR" && bash scripts/test_backend.sh --base-url "$BACKEND_BASE_URL")
}

run_frontend_login() {
  log "stage=frontend-login"
  check_url "$FRONTEND_BASE_URL/login" || check_url "$FRONTEND_BASE_URL"
  (cd "$ROOT_DIR" && FRONTEND_BASE_URL="$FRONTEND_BASE_URL" node scripts/test_frontend_login.mjs)
}

run_smoke() {
  run_doctor
  run_backend_unit
  run_api_smoke
  run_frontend_login
}

run_full() {
  run_doctor
  run_backend_unit
  run_backend_integration
  run_api_smoke
  run_frontend_login
}

if [[ $# -eq 0 ]]; then
  usage
  exit 1
fi

case "$1" in
  --list)
    cat <<'EOF'
doctor
backend-unit
backend-integration
api-smoke
frontend-login
smoke
full
EOF
    ;;
  --help|-h)
    usage
    ;;
  doctor)
    run_doctor
    ;;
  backend-unit)
    run_backend_unit
    ;;
  backend-integration)
    run_backend_integration
    ;;
  api-smoke)
    run_api_smoke
    ;;
  frontend-login)
    run_frontend_login
    ;;
  smoke)
    run_smoke
    ;;
  full)
    run_full
    ;;
  *)
    usage
    die "unknown stage: $1"
    ;;
esac
