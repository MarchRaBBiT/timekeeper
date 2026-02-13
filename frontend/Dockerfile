ARG RUST_VERSION=1.93
ARG CARGO_CHEF_VERSION=0.1.73
ARG WASM_PACK_VERSION=0.13.1

FROM rust:${RUST_VERSION} as chef
ARG CARGO_CHEF_VERSION
WORKDIR /app
RUN cargo install cargo-chef --version ${CARGO_CHEF_VERSION}
COPY . .
RUN cargo chef prepare --recipe-path recipe.json --bin timekeeper-frontend

FROM rust:${RUST_VERSION} as builder
ARG WASM_PACK_VERSION
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends mold binutils \
    && rm -rf /var/lib/apt/lists/*
RUN rustup target add wasm32-unknown-unknown
COPY --from=chef /usr/local/cargo/bin/cargo-chef /usr/local/cargo/bin/cargo-chef
COPY --from=chef /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo chef cook --release --recipe-path recipe.json --target wasm32-unknown-unknown --bin timekeeper-frontend

# Copy the full workspace to resolve workspace-inherited fields
COPY . .

# Install wasm-pack
RUN cargo install wasm-pack --version ${WASM_PACK_VERSION}

# Build frontend (WASM)
WORKDIR /app/frontend
ENV RUST_MIN_STACK=33554432 \
    CARGO_BUILD_JOBS=1 \
    RUSTFLAGS="-C codegen-units=1"
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    wasm-pack build --target web --out-dir pkg --release

FROM node:20-alpine as tailwind
WORKDIR /app/frontend
COPY frontend/package.json frontend/tailwind.config.js frontend/tailwind.input.css ./
COPY frontend/src ./src
RUN npm install
RUN npm run build:css

FROM nginx:alpine
WORKDIR /usr/share/nginx/html

# Place WASM pkg under /pkg to match index.html import path
COPY --from=builder /app/frontend/pkg/ ./pkg/
COPY --from=tailwind /app/frontend/assets/ ./assets/

# Static entry
COPY frontend/index.html ./index.html
COPY frontend/env.js ./env.js
COPY frontend/config.json ./config.json

# Nginx config with SPA fallback
COPY frontend/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80
