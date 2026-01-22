ALTER TABLE users ADD COLUMN password_changed_at TIMESTAMPTZ;

UPDATE users SET password_changed_at = updated_at WHERE password_changed_at IS NULL;

ALTER TABLE users ALTER COLUMN password_changed_at SET NOT NULL;
ALTER TABLE users ALTER COLUMN password_changed_at SET DEFAULT NOW();

ALTER TABLE archived_users ADD COLUMN password_changed_at TIMESTAMPTZ;

UPDATE archived_users SET password_changed_at = updated_at WHERE password_changed_at IS NULL;

ALTER TABLE archived_users ALTER COLUMN password_changed_at SET NOT NULL;
ALTER TABLE archived_users ALTER COLUMN password_changed_at SET DEFAULT NOW();

CREATE TABLE password_histories (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    password_hash TEXT NOT NULL,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_password_histories_user_id ON password_histories(user_id);
CREATE INDEX idx_password_histories_changed_at ON password_histories(changed_at);
