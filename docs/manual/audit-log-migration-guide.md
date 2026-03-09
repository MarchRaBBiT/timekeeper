# Audit Log Table Migration Guide

## Overview

Migration `018_create_audit_logs.sql` introduces a new audit log schema and renames the existing table to preserve data.

## Migration Contents

```sql
-- Rename existing table (preserves existing data)
ALTER TABLE IF EXISTS audit_logs RENAME TO audit_logs_legacy;

-- Create table with new schema
CREATE TABLE audit_logs (
    id TEXT PRIMARY KEY,
    occurred_at TIMESTAMP WITH TIME ZONE NOT NULL,
    actor_id TEXT,
    actor_type TEXT NOT NULL,
    event_type TEXT NOT NULL,
    target_type TEXT,
    target_id TEXT,
    result TEXT NOT NULL,
    error_code TEXT,
    metadata JSONB,
    ip TEXT,
    user_agent TEXT,
    request_id TEXT
);

-- Create indexes
CREATE INDEX idx_audit_logs_occurred_at ON audit_logs(occurred_at);
CREATE INDEX idx_audit_logs_actor_id ON audit_logs(actor_id);
CREATE INDEX idx_audit_logs_event_type ON audit_logs(event_type);
CREATE INDEX idx_audit_logs_target_id ON audit_logs(target_id);
CREATE INDEX idx_audit_logs_request_id ON audit_logs(request_id);
```

## Handling Existing Data

### Case 1: New Environment (No Existing Table)

The migration executes as-is and creates the new table.

### Case 2: Migration from Old Schema

1. Existing `audit_logs` table is renamed to `audit_logs_legacy`
2. New `audit_logs` table is created
3. Existing data is preserved in `audit_logs_legacy`

### Legacy Data Migration (If Needed)

To migrate old schema data to the new table, use the following SQL as reference:

```sql
-- Example: Adjust according to old schema columns
INSERT INTO audit_logs (id, occurred_at, actor_id, actor_type, event_type, target_type, target_id, result, error_code, metadata, ip, user_agent, request_id)
SELECT 
    id,
    occurred_at,
    actor_id,
    actor_type,
    event_type,
    target_type,
    target_id,
    result,
    error_code,
    metadata,
    ip,
    user_agent,
    request_id
FROM audit_logs_legacy;
```

### Removing Legacy Table

When migration is complete and legacy data is no longer needed:

```sql
DROP TABLE IF EXISTS audit_logs_legacy;
```

## Execution Steps

### Development Environment

```bash
cd backend
cargo sqlx migrate run
```

### Production Environment

1. Obtain database backup
2. Execute migration
3. Restart application
4. After verification, delete legacy table if needed

## Rollback

To rollback the migration (manual):

```sql
-- Drop new table
DROP TABLE IF EXISTS audit_logs;

-- Restore legacy table
ALTER TABLE IF EXISTS audit_logs_legacy RENAME TO audit_logs;
```

## Notes

- `audit_logs_legacy` is not referenced by the application, so consider deleting it after the migration period
- Testing in a staging environment before production execution is recommended
