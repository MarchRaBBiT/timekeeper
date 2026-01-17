#!/usr/bin/env bash
# Run backend integration tests using a real PostgreSQL container.

set -e

# Change to project root
cd "$(dirname "$0")/.."

echo "Starting test database..."
podman-compose -f docker-compose.test-db.yml up -d

# Wait for healthy state
echo "Waiting for database to be healthy..."
while [ "$(podman inspect --format='{{.State.Health.Status}}' timekeeper_test-db_1 2>/dev/null)" != "healthy" ]; do
    sleep 1
done

export TEST_DATABASE_URL="postgres://timekeeper_test:timekeeper_test@127.0.0.1:55432/timekeeper_test"

echo "Running tests..."
cd backend
cargo test "$@"

echo "Tests completed."
