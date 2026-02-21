#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: ./scripts/frontend.sh [build|start|stop|status|logs] [options]

Commands:
  build    Build frontend assets (Tailwind + wasm-pack)
  start    Build and start local static server (default)
  stop     Stop running frontend server
  status   Show frontend server status
  logs     Tail frontend server logs

Options:
  --port N               HTTP port (default: 8000)
  --release              Build in release mode
  --connect-src VALUE    CSP connect-src for generated dev index.html
                         (default: $FRONTEND_CSP_CONNECT_SRC or "'self' http://localhost:3000")
  --help                 Show this help
USAGE
}

CMD="start"
PORT=8000
RELEASE=0
CONNECT_SRC="${FRONTEND_CSP_CONNECT_SRC:-"'self' http://localhost:3000"}"

if [[ $# -gt 0 ]]; then
  case "$1" in
    build|start|stop|status|logs)
      CMD="$1"
      shift
      ;;
  esac
fi

while [[ $# -gt 0 ]]; do
  case "$1" in
    --port)
      PORT="$2"
      shift 2
      ;;
    --release)
      RELEASE=1
      shift
      ;;
    --connect-src)
      CONNECT_SRC="$2"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage
      exit 1
      ;;
  esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
FRONTEND_ROOT="$PROJECT_ROOT/frontend"
PID_FILE="$PROJECT_ROOT/.frontend.pid"
LOG_FILE="$FRONTEND_ROOT/frontend-dev.log"
SERVE_ROOT="$FRONTEND_ROOT/.serve"

find_python() {
  if command -v python3 >/dev/null 2>&1; then
    echo "python3"
  elif command -v python >/dev/null 2>&1; then
    echo "python"
  else
    echo "python3 or python is required." >&2
    exit 1
  fi
}

build_tailwind() {
  cd "$FRONTEND_ROOT"

  if ! command -v node >/dev/null 2>&1; then
    echo "Node.js is not installed. Please install Node.js to build Tailwind CSS." >&2
    exit 1
  fi

  local tailwind_bin="$FRONTEND_ROOT/node_modules/.bin/tailwindcss"
  local input="$FRONTEND_ROOT/tailwind.input.css"
  local output_dir="$FRONTEND_ROOT/assets"
  local output="$output_dir/tailwind.css"

  if [[ ! -x "$tailwind_bin" ]]; then
    echo "Tailwind CSS is not installed. Please run: npm install (in frontend/)" >&2
    exit 1
  fi

  mkdir -p "$output_dir"
  "$tailwind_bin" -i "$input" -o "$output" --minify
}

build_frontend() {
  cd "$FRONTEND_ROOT"
  build_tailwind

  if ! command -v wasm-pack >/dev/null 2>&1; then
    echo "wasm-pack is not installed. Please run: cargo install wasm-pack" >&2
    exit 1
  fi

  local mode="--dev"
  if [[ "$RELEASE" -eq 1 ]]; then
    mode="--release"
  fi

  wasm-pack build --target web --out-dir pkg "$mode"
}

escape_for_sed_replacement() {
  printf '%s' "$1" | sed -e 's/[\/&]/\\&/g'
}

create_dev_serve_root() {
  local template_path="$FRONTEND_ROOT/index.html.template"
  local index_path="$SERVE_ROOT/index.html"

  if [[ ! -f "$template_path" ]]; then
    echo "Missing template file: $template_path" >&2
    exit 1
  fi
  if [[ ! -d "$FRONTEND_ROOT/pkg" ]]; then
    echo "Missing frontend/pkg. Run build first." >&2
    exit 1
  fi
  if [[ ! -d "$FRONTEND_ROOT/assets" ]]; then
    echo "Missing frontend/assets. Run build first." >&2
    exit 1
  fi

  rm -rf "$SERVE_ROOT"
  mkdir -p "$SERVE_ROOT"

  cp -R "$FRONTEND_ROOT/pkg" "$SERVE_ROOT/pkg"
  cp -R "$FRONTEND_ROOT/assets" "$SERVE_ROOT/assets"
  cp "$FRONTEND_ROOT/env.js" "$SERVE_ROOT/env.js"
  cp "$FRONTEND_ROOT/config.json" "$SERVE_ROOT/config.json"

  local escaped
  escaped="$(escape_for_sed_replacement "$CONNECT_SRC")"
  sed "s/__CSP_CONNECT_SRC__/$escaped/g" "$template_path" > "$index_path"
}

start_frontend() {
  if [[ -f "$PID_FILE" ]]; then
    echo "PID file exists. Use 'stop' first if process is stale." >&2
  fi

  build_frontend
  create_dev_serve_root

  local py
  py="$(find_python)"

  (
    cd "$SERVE_ROOT"
    nohup "$py" -m http.server "$PORT" > "$LOG_FILE" 2>&1 &
    echo $! > "$PID_FILE"
  )

  local pid
  pid="$(cat "$PID_FILE")"
  echo "Frontend started. PID=$pid. http://localhost:$PORT Logs: $LOG_FILE"
}

stop_frontend() {
  if [[ ! -f "$PID_FILE" ]]; then
    echo "No PID file; nothing to stop"
    return
  fi

  local pid
  pid="$(head -n 1 "$PID_FILE" || true)"
  if [[ -n "$pid" ]] && kill -0 "$pid" >/dev/null 2>&1; then
    kill "$pid" || true
    echo "Stopped frontend (PID=$pid)"
  else
    echo "PID file is stale ($pid)"
  fi
  rm -f "$PID_FILE"
}

status_frontend() {
  if [[ ! -f "$PID_FILE" ]]; then
    echo "Status: not running"
    return
  fi

  local pid
  pid="$(head -n 1 "$PID_FILE" || true)"
  if [[ -n "$pid" ]] && kill -0 "$pid" >/dev/null 2>&1; then
    echo "Status: running (PID=$pid)"
  else
    echo "Status: stale PID file ($pid)"
  fi
}

logs_frontend() {
  if [[ -f "$LOG_FILE" ]]; then
    tail -n 200 -f "$LOG_FILE"
  else
    echo "No log file yet: $LOG_FILE"
  fi
}

case "$CMD" in
  build)
    build_frontend
    ;;
  start)
    start_frontend
    ;;
  stop)
    stop_frontend
    ;;
  status)
    status_frontend
    ;;
  logs)
    logs_frontend
    ;;
  *)
    usage
    exit 1
    ;;
esac
