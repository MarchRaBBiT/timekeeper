# Scaling Guide

This document describes how to scale the Timekeeper system for higher load.

## Database Scaling

### Connection Pooling
The system uses `sqlx` with connection pooling. Tune these variables based on your DB capacity:
- `DB_MAX_CONNECTIONS`: Maximum number of connections in the pool (default: 10).
- `DB_MIN_CONNECTIONS`: Minimum number of idle connections (default: 2).
- `DB_ACQUIRE_TIMEOUT`: Seconds to wait for a connection (default: 30).

### Read Replicas
For horizontal read scaling, set `READ_DATABASE_URL`.
- GET requests will automatically use the read replica.
- Mutations (POST/PUT/DELETE) and auth-critical checks always use the primary.
- Feature flag: `FEATURE_READ_REPLICA_ENABLED=true` (default).

## Caching (Redis)

### Token Caching
Active JWT access tokens are cached in Redis to avoid DB hits on every request.
- `REDIS_URL`: e.g., `redis://localhost:6379`.
- `REDIS_POOL_SIZE`: Max connections to Redis (default: 10).
- `FEATURE_REDIS_CACHE_ENABLED=true` (default).

### Cache-Aside Pattern
The system checks Redis first. If the token is not found, it falls back to PostgreSQL and backfills Redis.

## Monitoring
The system emits tracing spans for latency measurement:
- `db_pool_connect`: Database connection acquisition.
- `redis_cache_token`: Caching operations.
- `redis_is_token_active`: Token validation latency.
