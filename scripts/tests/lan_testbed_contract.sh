#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
compose_file="$repo_root/docker-compose.lan-test.yml"
launcher="$repo_root/scripts/lan-testbed.sh"
guide="$repo_root/docs/manual/lan-testbed.md"

fail() {
  printf 'lan-testbed contract failed: %s\n' "$*" >&2
  exit 1
}

for path in "$compose_file" "$launcher" "$guide"; do
  [[ -f "$path" ]] || fail "missing ${path#"$repo_root/"}"
done

compose="$(<"$compose_file")"
launcher_text="$(<"$launcher")"

backend_block="$(awk '/^  backend:/{capture=1} /^  frontend:/{capture=0} capture{print}' "$compose_file")"
if grep -Eq '^[[:space:]]+ports:' <<<"$backend_block"; then
  fail "backend must not publish a host port"
fi
grep -Fq '${LAN_TESTBED_IP}:${LAN_TESTBED_HTTPS_PORT:-8443}:443' <<<"$compose" \
  || fail "frontend HTTPS port is not configurable"
if grep -Fq ':80"' <<<"$compose"; then
  fail "frontend HTTP must not be published"
fi

for forbidden in '5432:5432' '6379:6379' '5050:80'; do
  if grep -Fq "$forbidden" <<<"$compose"; then
    fail "infrastructure port must not be published: $forbidden"
  fi
done

grep -Eq 'COOKIE_SECURE:[[:space:]]+"?true"?' <<<"$compose" \
  || fail "secure auth cookies must remain enabled"
grep -Fq 'CORS_ALLOW_ORIGINS: https://${LAN_TESTBED_HOST}:${LAN_TESTBED_HTTPS_PORT:-8443}' <<<"$compose" \
  || fail "LAN origin must be explicit"
grep -Fq 'RATE_LIMIT_IP_MAX_REQUESTS: ${LAN_TESTBED_RATE_LIMIT_IP_MAX_REQUESTS:-15}' <<<"$compose" \
  || fail "safe rate-limit default is missing"
grep -Fq 'TLS_SAN: ${LAN_TESTBED_TLS_SAN}' <<<"$compose" \
  || fail "certificate SAN must be passed to the frontend"
grep -Fq '${LAN_TESTBED_TLS_DIR}:/etc/nginx/tls:Z' <<<"$compose" \
  || fail "certificate must persist outside the container"

grep -Fq 'LAN_TESTBED_HOST' <<<"$launcher_text" \
  || fail "launcher must resolve a LAN host"
grep -Fq 'LAN_TESTBED_IP' <<<"$launcher_text" \
  || fail "launcher must resolve a LAN IP"
grep -Fq 'podman compose' <<<"$launcher_text" \
  || fail "launcher must use the repository Podman workflow"
grep -Fq "printf 'Browser: https://%s:%s" <<<"$launcher_text" \
  || fail "launcher must report the browser URL"

printf 'lan-testbed contract: pass\n'
