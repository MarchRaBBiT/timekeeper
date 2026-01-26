CREATE UNIQUE INDEX idx_active_sessions_refresh_token_id_unique
    ON active_sessions(refresh_token_id);
