#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
compose_file="$repo_root/docker-compose.lan-test.yml"
secret_file="$repo_root/.lan-testbed.env"
action="${1:-up}"

die() {
  printf 'lan-testbed: %s\n' "$*" >&2
  exit 1
}

command -v podman >/dev/null 2>&1 || die "podman is required"
podman compose version >/dev/null 2>&1 || die "podman compose is required"

LAN_TESTBED_HOST="${LAN_TESTBED_HOST:-$(hostname 2>/dev/null || true)}"
[[ -n "$LAN_TESTBED_HOST" ]] || die "set LAN_TESTBED_HOST to this server's LAN hostname or IP"
[[ "$LAN_TESTBED_HOST" =~ ^[A-Za-z0-9.-]+$ ]] \
  || die "LAN_TESTBED_HOST may only contain letters, digits, dots, and hyphens"

if [[ -z "${LAN_TESTBED_IP:-}" ]]; then
  LAN_TESTBED_IP="$(
    hostname -I 2>/dev/null \
      | tr ' ' '\n' \
      | awk '/^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$/ && $0 !~ /^127\./ { print; exit }'
  )"
fi
[[ -n "$LAN_TESTBED_IP" ]] || die "set LAN_TESTBED_IP to this server's non-loopback IPv4 address"
[[ "$LAN_TESTBED_IP" =~ ^[0-9]{1,3}(\.[0-9]{1,3}){3}$ ]] \
  || die "LAN_TESTBED_IP must be an IPv4 address"

LAN_TESTBED_HTTPS_PORT="${LAN_TESTBED_HTTPS_PORT:-8443}"
LAN_TESTBED_TLS_SAN="DNS:${LAN_TESTBED_HOST},IP:${LAN_TESTBED_IP},DNS:localhost,IP:127.0.0.1"
LAN_TESTBED_TLS_DIR="${LAN_TESTBED_TLS_DIR:-$repo_root/target/lan-testbed/tls}"
mkdir -p "$LAN_TESTBED_TLS_DIR"
existing_cert="$LAN_TESTBED_TLS_DIR/localhost.crt"
if [[ -f "$existing_cert" ]]; then
  openssl x509 -in "$existing_cert" -noout -checkhost "$LAN_TESTBED_HOST" >/dev/null 2>&1 \
    || die "existing certificate does not cover $LAN_TESTBED_HOST; remove target/lan-testbed/tls and retry"
  openssl x509 -in "$existing_cert" -noout -checkip "$LAN_TESTBED_IP" >/dev/null 2>&1 \
    || die "existing certificate does not cover $LAN_TESTBED_IP; remove target/lan-testbed/tls and retry"
fi

if [[ -f "$secret_file" ]]; then
  # shellcheck disable=SC1090
  source "$secret_file"
else
  command -v openssl >/dev/null 2>&1 || die "openssl is required to generate the test secret"
  umask 077
  LAN_TESTBED_JWT_SECRET="$(openssl rand -hex 32)"
  printf 'LAN_TESTBED_JWT_SECRET=%s\n' "$LAN_TESTBED_JWT_SECRET" >"$secret_file"
fi
[[ ${#LAN_TESTBED_JWT_SECRET} -ge 32 ]] || die "LAN_TESTBED_JWT_SECRET must be at least 32 characters"

export LAN_TESTBED_HOST LAN_TESTBED_IP LAN_TESTBED_HTTPS_PORT
export LAN_TESTBED_TLS_SAN LAN_TESTBED_TLS_DIR LAN_TESTBED_JWT_SECRET

compose() {
  podman compose --project-name timekeeper-lan-testbed -f "$compose_file" "$@"
}

case "$action" in
  up)
    compose up --build --detach
    compose ps
    printf '\nLAN testbed is ready when all services are running.\n'
    printf 'Browser: https://%s:%s\n' "$LAN_TESTBED_HOST" "$LAN_TESTBED_HTTPS_PORT"
    printf 'Login: admin / admin123 (change it if this LAN is not trusted)\n'
    printf 'The certificate is self-signed; accept it only for this testbed.\n'
    printf 'Certificate for optional client trust: %s/localhost.crt\n' "$LAN_TESTBED_TLS_DIR"
    ;;
  down)
    compose down
    ;;
  reset)
    printf 'This deletes only the timekeeper-lan-testbed database and Redis volumes.\n'
    compose down --volumes
    ;;
  status)
    compose ps
    ;;
  logs)
    compose logs --follow
    ;;
  check)
    curl --fail --silent --show-error --insecure \
      "https://${LAN_TESTBED_IP}:${LAN_TESTBED_HTTPS_PORT}/api/config/timezone"
    printf '\nLAN testbed check: pass\n'
    ;;
  *)
    die "usage: scripts/lan-testbed.sh [up|down|reset|status|logs|check]"
    ;;
esac
