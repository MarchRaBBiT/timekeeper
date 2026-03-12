#!/bin/sh
set -eu

TLS_DIR="/etc/nginx/tls"
CERT_FILE="${TLS_CERT_FILE:-$TLS_DIR/localhost.crt}"
KEY_FILE="${TLS_KEY_FILE:-$TLS_DIR/localhost.key}"
TLS_DAYS="${TLS_CERT_DAYS:-3650}"
TLS_CN="${TLS_CERT_CN:-localhost}"
API_BASE_URL_VALUE="${API_BASE_URL:-/api}"

mkdir -p "$TLS_DIR"

if [ ! -f "$CERT_FILE" ] || [ ! -f "$KEY_FILE" ]; then
  openssl req \
    -x509 \
    -nodes \
    -days "$TLS_DAYS" \
    -newkey rsa:2048 \
    -keyout "$KEY_FILE" \
    -out "$CERT_FILE" \
    -subj "/CN=$TLS_CN" \
    -addext "subjectAltName=DNS:localhost,IP:127.0.0.1"
fi

cat > /usr/share/nginx/html/env.js <<EOF
window.__TIMEKEEPER_ENV = Object.assign({}, window.__TIMEKEEPER_ENV || {}, {
  API_BASE_URL: "${API_BASE_URL_VALUE}"
});
EOF
