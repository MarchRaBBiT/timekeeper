ALTER TABLE users
    ADD COLUMN failed_login_attempts INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN locked_until TIMESTAMPTZ,
    ADD COLUMN lock_reason TEXT,
    ADD COLUMN lockout_count INTEGER NOT NULL DEFAULT 0;

ALTER TABLE archived_users
    ADD COLUMN failed_login_attempts INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN locked_until TIMESTAMPTZ,
    ADD COLUMN lock_reason TEXT,
    ADD COLUMN lockout_count INTEGER NOT NULL DEFAULT 0;
