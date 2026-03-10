ALTER TABLE users
    ADD COLUMN last_login_failure_at TIMESTAMPTZ;

ALTER TABLE archived_users
    ADD COLUMN last_login_failure_at TIMESTAMPTZ;
