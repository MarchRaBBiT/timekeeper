ALTER TABLE active_sessions
    ADD COLUMN access_jti TEXT;

CREATE UNIQUE INDEX idx_active_sessions_access_jti_unique
    ON active_sessions(access_jti);
