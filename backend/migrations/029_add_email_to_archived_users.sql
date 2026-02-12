-- Add email column to archived_users so restore can reinsert into users(email NOT NULL)
ALTER TABLE archived_users ADD COLUMN email TEXT;

-- Backfill existing archived rows with a deterministic unique placeholder.
UPDATE archived_users
SET email = 'archived+' || id || '@invalid.local'
WHERE email IS NULL;

ALTER TABLE archived_users ALTER COLUMN email SET NOT NULL;

CREATE INDEX idx_archived_users_email ON archived_users(email);
