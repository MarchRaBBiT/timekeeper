-- Add email column to users
ALTER TABLE users ADD COLUMN email TEXT;

-- Update existing default admin if exists
UPDATE users SET email = 'admin@example.com' WHERE username = 'admin' AND email IS NULL;

-- For any other existing users, set placeholder to allow NOT NULL constraint
UPDATE users SET email = username || '@example.com' WHERE email IS NULL;

-- Now add constraints
ALTER TABLE users ALTER COLUMN email SET NOT NULL;
ALTER TABLE users ADD CONSTRAINT users_email_key UNIQUE (email);

-- Create password_resets table
CREATE TABLE password_resets (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    used_at TIMESTAMPTZ
);

-- Index for cleanup and lookup
CREATE INDEX idx_password_resets_user_id ON password_resets(user_id);
CREATE INDEX idx_password_resets_token_hash ON password_resets(token_hash);
