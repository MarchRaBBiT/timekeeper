ARG RUST_VERSION=1.93
ARG CARGO_CHEF_VERSION=0.1.73

FROM rust:${RUST_VERSION} as chef
ARG CARGO_CHEF_VERSION
WORKDIR /app
RUN cargo install cargo-chef --version ${CARGO_CHEF_VERSION}
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM rust:${RUST_VERSION} as builder
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends mold binutils \
    && rm -rf /var/lib/apt/lists/*
COPY --from=chef /usr/local/cargo/bin/cargo-chef /usr/local/cargo/bin/cargo-chef
COPY --from=chef /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo chef cook --release --recipe-path recipe.json

# Copy workspace manifests first (better layer caching)
COPY Cargo.toml ./
COPY backend/Cargo.toml backend/Cargo.toml
COPY frontend/Cargo.toml frontend/Cargo.toml

# Copy full workspace
COPY . .

# Build only backend binary in release
ENV RUST_MIN_STACK=16777216 \
    CARGO_BUILD_JOBS=1
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build -p timekeeper-backend --release

FROM debian:trixie-slim
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libpq5 \
    && rm -rf /var/lib/apt/lists/* \
    && update-ca-certificates

# Copy binary from builder
COPY --from=builder /app/target/release/timekeeper-backend /usr/local/bin/timekeeper-backend

# Copy migrations (embedded at compile time, but useful to keep around)
COPY backend/migrations ./migrations

ENV RUST_LOG=info \
    DATABASE_URL=postgres://timekeeper:timekeeper@db:5432/timekeeper \
    APP_TIMEZONE=Asia/Tokyo

EXPOSE 3000
CMD ["timekeeper-backend"]
