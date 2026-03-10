# Operational Runbook

## Redis Failure

### Symptoms
- Increased latency in API requests (especially authenticated ones).
- Tracing shows `redis_cache_token` or `redis_is_token_active` errors.
- System fallback to PostgreSQL for token validation.

### Recovery
The system implements a graceful fallback to PostgreSQL. If Redis is down, authentication will still work but with higher DB load.

1. **Verify Redis Status**: Check if the Redis container/server is running.
2. **Restore Redis**: Restart the Redis service.
3. **Cache Re-population**: The cache will be re-populated as users log in or perform requests (cache-aside pattern).

### Configuration
Disable caching if Redis remains unstable:
`FEATURE_REDIS_CACHE_ENABLED=false`

## Database Connection Issues

### Symptoms
- `db_pool_connect` errors in logs.
- API returns 500 status codes.

### Recovery
1. **Check primary DB**: Ensure PostgreSQL is reachable.
2. **Read Replica Fallback**: If `READ_DATABASE_URL` is configured, the system will still use it for GETs. If the primary is down, writes will fail but reads might still work.
3. **Scale Pool**: Increase `DB_MAX_CONNECTIONS` if the pool is exhausted.
