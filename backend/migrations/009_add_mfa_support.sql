ALTER TABLE users
    ADD COLUMN IF NOT EXISTS mfa_secret TEXT,
    ADD COLUMN IF NOT EXISTS mfa_enabled_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_users_mfa_enabled_at ON users (mfa_enabled_at);
