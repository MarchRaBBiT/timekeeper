#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BACKEND_BASE_URL="${BACKEND_BASE_URL:-http://localhost:3000}"
FRONTEND_BASE_URL="${FRONTEND_BASE_URL:-https://localhost:8080}"
BACKEND_READINESS_PATH="${BACKEND_READINESS_PATH:-/api/config/timezone}"

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
  bash scripts/harness.sh docs-check
  bash scripts/harness.sh fmt-check
  bash scripts/harness.sh backend-unit
  bash scripts/harness.sh backend-integration
  bash scripts/harness.sh clippy-backend
  bash scripts/harness.sh clippy-frontend
  bash scripts/harness.sh lint
  bash scripts/harness.sh api-smoke
  bash scripts/harness.sh frontend-login
  bash scripts/harness.sh smoke
  bash scripts/harness.sh full

Environment:
  BACKEND_BASE_URL   default: http://localhost:3000
  BACKEND_READINESS_PATH default: /api/config/timezone
  FRONTEND_BASE_URL  default: https://localhost:8080
EOF
}

require_cmd() {
  local cmd="$1"
  command -v "$cmd" >/dev/null 2>&1 || die "missing command: $cmd"
}

check_url() {
  local url="$1"
  if [[ "$url" == https://* ]]; then
    curl -k -fsS --max-time 5 "$url" >/dev/null
  else
    curl -fsS --max-time 5 "$url" >/dev/null
  fi
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

run_docs_check() {
  log "stage=docs-check"
  local required_files=(
    "AGENTS.md"
    "docs/manual/CODING_AGENT.md"
    "docs/manual/HARNESS.md"
    "docs/design-docs/harness-engineering.md"
    "docs/design-docs/rebuild-architecture.md"
    "docs/exec-plans/active/EP-20260610-rebuild-architecture-harness.md"
    ".agent/PLANS.md"
  )

  local file
  for file in "${required_files[@]}"; do
    [[ -f "$ROOT_DIR/$file" ]] || die "missing harness source file: $file"
  done

  grep -q "rebuild-architecture.md" "$ROOT_DIR/AGENTS.md" || die "AGENTS.md does not reference rebuild architecture"
  grep -q "PostgreSQL" "$ROOT_DIR/docs/design-docs/rebuild-architecture.md" || die "rebuild architecture does not state database direction"
  grep -q "docs-check" "$ROOT_DIR/docs/manual/HARNESS.md" || die "HARNESS.md does not document docs-check"
  grep -q "docs-check" "$ROOT_DIR/scripts/harness.sh" || die "harness script does not expose docs-check"

  local stable_docs=(
    "AGENTS.md"
    "docs/manual/CODING_AGENT.md"
    "docs/manual/HARNESS.md"
    "docs/design-docs/harness-engineering.md"
    "docs/design-docs/rebuild-architecture.md"
  )

  for file in "${stable_docs[@]}"; do
    if grep -Eq '(^|[^[:alnum:]_])jj([^[:alnum:]_]|$)' "$ROOT_DIR/$file"; then
      die "stable harness doc still references jj workflow: $file"
    fi
  done
}

run_fmt_check() {
  log "stage=fmt-check"
  (cd "$ROOT_DIR" && cargo fmt --all --check)
}

run_backend_unit() {
  log "stage=backend-unit"
  (cd "$ROOT_DIR" && cargo test -p timekeeper-backend --lib)
}

run_backend_integration() {
  log "stage=backend-integration"
  # Podman socket が未起動の場合は activate する（Docker 未導入環境向け）
  if command -v systemctl &>/dev/null && command -v podman &>/dev/null; then
    if ! systemctl --user is-active --quiet podman.socket 2>/dev/null; then
      systemctl --user start podman.socket 2>/dev/null || true
    fi
    export DOCKER_HOST="unix:///run/user/$(id -u)/podman/podman.sock"
  fi
  (cd "$ROOT_DIR" && cargo test -p timekeeper-backend --tests)
}

run_clippy_backend() {
  log "stage=clippy-backend"
  (cd "$ROOT_DIR" && cargo clean -p utoipa-swagger-ui)
  (cd "$ROOT_DIR" && cargo clippy -p timekeeper-backend --all-targets -- -D warnings)
}

run_clippy_frontend() {
  log "stage=clippy-frontend"
  (cd "$ROOT_DIR" && cargo clean -p utoipa-swagger-ui)
  (cd "$ROOT_DIR" && cargo clippy -p timekeeper-frontend --all-targets -- -D warnings)
}

run_lint() {
  run_docs_check
  run_fmt_check
  (cd "$ROOT_DIR" && cargo clean -p utoipa-swagger-ui)
  log "stage=clippy-workspace"
  (cd "$ROOT_DIR" && cargo clippy --all-targets -- -D warnings)
}

run_api_smoke() {
  log "stage=api-smoke"
  check_url "${BACKEND_BASE_URL}${BACKEND_READINESS_PATH}" || die "backend readiness check failed at ${BACKEND_BASE_URL}${BACKEND_READINESS_PATH}"
  (cd "$ROOT_DIR" && bash scripts/test_backend.sh --base-url "$BACKEND_BASE_URL")
}

run_frontend_login() {
  log "stage=frontend-login"
  check_url "$FRONTEND_BASE_URL/login" || check_url "$FRONTEND_BASE_URL" || die "frontend not reachable at $FRONTEND_BASE_URL"
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
  run_lint
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
docs-check
fmt-check
backend-unit
backend-integration
clippy-backend
clippy-frontend
lint
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
  docs-check)
    run_docs_check
    ;;
  fmt-check)
    run_fmt_check
    ;;
  backend-unit)
    run_backend_unit
    ;;
  backend-integration)
    run_backend_integration
    ;;
  clippy-backend)
    run_clippy_backend
    ;;
  clippy-frontend)
    run_clippy_frontend
    ;;
  lint)
    run_lint
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
