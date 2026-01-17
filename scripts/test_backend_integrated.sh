#!/usr/bin/env bash
# Run backend integration tests using a real PostgreSQL container.

set -e

# Change to project root
cd "$(dirname "$0")/.."

echo "Starting test database..."
podman-compose -f docker-compose.test-db.yml up -d

# Wait for healthy state
echo "Waiting for database to be healthy..."
MAX_WAIT_SECONDS=60
ELAPSED_SECONDS=0
while [ "$(podman inspect --format='{{.State.Health.Status}}' timekeeper_test-db_1 2>/dev/null)" != "healthy" ]; do
    sleep 1
    ELAPSED_SECONDS=$((ELAPSED_SECONDS + 1))
    if [ "$ELAPSED_SECONDS" -ge "$MAX_WAIT_SECONDS" ]; then
        echo "Database failed to become healthy within ${MAX_WAIT_SECONDS}s"
        exit 1
    fi
done

export TEST_DATABASE_URL="postgres://timekeeper_test:timekeeper_test@127.0.0.1:55432/timekeeper_test"

echo "Running tests..."
cd backend
cargo test "$@"

echo "Tests completed."
