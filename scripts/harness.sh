#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BACKEND_BASE_URL="${BACKEND_BASE_URL:-http://localhost:3000}"
FRONTEND_BASE_URL="${FRONTEND_BASE_URL:-https://localhost:8080}"
BACKEND_READINESS_PATH="${BACKEND_READINESS_PATH:-/api/config/timezone}"
# Suite-level serial execution policy (tech-debt-tracker.md item #5, Recommended Fix 3):
# every backend integration test file compiles into its own `cargo test` binary, and a single
# `cargo test --tests` invocation already runs those binaries one at a time (verified: two
# probe test binaries executed sequentially, never overlapping). The remaining race window is
# *cross-invocation*: two separate `cargo test` / harness.sh runs (e.g. two terminals, or
# `backend-integration` and `backend-security-smoke` launched at the same time) pointed at the
# same shared external Postgres (docker-compose test-db on 127.0.0.1:55432, as used by
# scripts/test_backend_integrated.sh). This lock file serializes exactly that case.
BACKEND_INTEGRATION_LOCK="${BACKEND_INTEGRATION_LOCK:-$ROOT_DIR/target/harness-locks/backend-integration.lock}"
# Focused files for the `backend-security-smoke` stage: auth / lockout / rate-limit / mfa /
# session hardening surfaces (tech-debt-tracker.md item #5, Recommended Fix 4).
BACKEND_SECURITY_SMOKE_TESTS=(
  auth_flow_api
  auth_lockout_redis_integration
  rate_limit_redis_integration
  password_api
  password_reset_api
  mfa_api
  session_api
  active_session_repo
)

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
  bash scripts/harness.sh backend-security-smoke
  bash scripts/harness.sh worker-once
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
  BACKEND_INTEGRATION_LOCK default: target/harness-locks/backend-integration.lock
  DATABASE_URL       required for worker-once (live Postgres)
  REDIS_URL          required for worker-once (live Redis)
  JWT_SECRET         required for worker-once (>=32 chars, same as backend runtime)
EOF
}

require_cmd() {
  local cmd="$1"
  command -v "$cmd" >/dev/null 2>&1 || die "missing command: $cmd"
}

require_env() {
  local name="$1"
  if [[ -z "${!name:-}" ]]; then
    die "missing environment variable: $name (see docs/manual/HARNESS.md, worker-once section)"
  fi
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

ensure_podman_socket() {
  # Podman socket が未起動の場合は activate する（Docker 未導入環境向け）
  if command -v systemctl &>/dev/null && command -v podman &>/dev/null; then
    if ! systemctl --user is-active --quiet podman.socket 2>/dev/null; then
      systemctl --user start podman.socket 2>/dev/null || true
    fi
    export DOCKER_HOST="unix:///run/user/$(id -u)/podman/podman.sock"
  fi
}

# Runs "$@" as a command line, serialized against BACKEND_INTEGRATION_LOCK so that concurrent
# harness/cargo-test invocations against a shared external test database cannot race on
# TRUNCATE-based fixtures (see the "Suite Execution Model" note near the top of this file and
# docs/manual/HARNESS.md).
with_backend_integration_lock() {
  mkdir -p "$(dirname "$BACKEND_INTEGRATION_LOCK")"
  if command -v flock >/dev/null 2>&1; then
    flock "$BACKEND_INTEGRATION_LOCK" -c "$*"
  else
    log "flock not available; running without cross-invocation DB lock (see docs/manual/HARNESS.md)"
    (cd "$ROOT_DIR" && eval "$*")
  fi
}

run_backend_integration() {
  log "stage=backend-integration"
  ensure_podman_socket
  # --no-fail-fast: without it, cargo stops running further *.rs test binaries as soon as one
  # binary reports a failure, which (given ~50 independent test binaries) hides most of the
  # suite's signal behind a single unrelated failure. Fragility of the harness is exactly what
  # this stage exists to reduce, so we always run every test binary and report the full set of
  # failures in one pass.
  with_backend_integration_lock "cd '$ROOT_DIR' && cargo test -p timekeeper-backend --tests --no-fail-fast"
}

run_backend_security_smoke() {
  log "stage=backend-security-smoke"
  ensure_podman_socket
  local test_args=()
  local name
  for name in "${BACKEND_SECURITY_SMOKE_TESTS[@]}"; do
    test_args+=(--test "$name")
  done
  with_backend_integration_lock "cd '$ROOT_DIR' && cargo test -p timekeeper-backend ${test_args[*]} --no-fail-fast"
}

run_worker_once() {
  log "stage=worker-once"
  # lockout_notification_worker connects directly to a live Postgres + Redis (no HTTP server
  # involved), so this stage needs the same env vars the binary itself requires: DATABASE_URL /
  # JWT_SECRET (via Config::load()) and REDIS_URL (the worker exits with an error if it is
  # unset — see backend/src/bin/lockout_notification_worker.rs). `--once` makes the worker drain
  # at most one due retry batch, process at most one queued job (if any), then exit instead of
  # looping forever, which is what makes this usable as a bounded harness smoke stage. See
  # docs/manual/RUNBOOK.md "Notification Worker Operations" for the operational model this stage
  # exercises (retry / DLQ / drain boundaries, tech-debt-tracker.md item #7).
  require_env DATABASE_URL
  require_env REDIS_URL
  require_env JWT_SECRET
  (cd "$ROOT_DIR" && cargo run --bin lockout_notification_worker -- --once)
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
backend-security-smoke
worker-once
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
  backend-security-smoke)
    run_backend_security_smoke
    ;;
  worker-once)
    run_worker_once
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
