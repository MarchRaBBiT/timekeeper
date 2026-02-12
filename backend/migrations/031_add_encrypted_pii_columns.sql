-- Add encrypted PII columns and lookup hash columns.
ALTER TABLE users
    ADD COLUMN full_name_enc TEXT,
    ADD COLUMN email_enc TEXT,
    ADD COLUMN email_hash TEXT,
    ADD COLUMN mfa_secret_enc TEXT,
    ADD COLUMN pii_key_version SMALLINT NOT NULL DEFAULT 1;

ALTER TABLE archived_users
    ADD COLUMN full_name_enc TEXT,
    ADD COLUMN email_enc TEXT,
    ADD COLUMN email_hash TEXT,
    ADD COLUMN mfa_secret_enc TEXT,
    ADD COLUMN pii_key_version SMALLINT NOT NULL DEFAULT 1;

CREATE UNIQUE INDEX IF NOT EXISTS idx_users_email_hash_unique ON users(email_hash) WHERE email_hash IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_archived_users_email_hash ON archived_users(email_hash);
