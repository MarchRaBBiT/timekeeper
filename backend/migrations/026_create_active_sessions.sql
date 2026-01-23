CREATE TABLE active_sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    refresh_token_id TEXT NOT NULL REFERENCES refresh_tokens(id) ON DELETE CASCADE,
    device_label TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_seen_at TIMESTAMP WITH TIME ZONE,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE INDEX idx_active_sessions_user_id ON active_sessions(user_id);
CREATE INDEX idx_active_sessions_refresh_token_id ON active_sessions(refresh_token_id);
CREATE INDEX idx_active_sessions_expires_at ON active_sessions(expires_at);
