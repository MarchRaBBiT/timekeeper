FROM rust:1.85 as builder
WORKDIR /app
# Copy the full workspace to resolve workspace-inherited fields
COPY . .

# Install wasm-pack
RUN cargo install wasm-pack

# Build frontend (WASM)
WORKDIR /app/frontend
ENV RUST_MIN_STACK=16777216 \
    CARGO_BUILD_JOBS=1
RUN wasm-pack build --target web --out-dir pkg --release

FROM nginx:alpine
WORKDIR /usr/share/nginx/html

# Place WASM pkg under /pkg to match index.html import path
COPY --from=builder /app/frontend/pkg/ ./pkg/

# Static entry
COPY frontend/index.html ./index.html
COPY frontend/config.json ./config.json

# Nginx config with SPA fallback
COPY frontend/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80
