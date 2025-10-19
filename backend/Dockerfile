FROM rust:1.85 as builder
WORKDIR /app

# Copy workspace manifests first (better layer caching)
COPY Cargo.toml ./
COPY backend/Cargo.toml backend/Cargo.toml
COPY frontend/Cargo.toml frontend/Cargo.toml

# Copy full workspace
COPY . .

# Build only backend binary in release
ENV RUST_MIN_STACK=16777216 \
    CARGO_BUILD_JOBS=1
RUN cargo build -p timekeeper-backend --release

FROM debian:bookworm-slim
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
