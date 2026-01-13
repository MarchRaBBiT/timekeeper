CREATE TABLE consent_logs (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    purpose TEXT NOT NULL,
    policy_version TEXT NOT NULL,
    consented_at TIMESTAMP WITH TIME ZONE NOT NULL,
    ip TEXT,
    user_agent TEXT,
    request_id TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_consent_logs_user_id ON consent_logs(user_id);
CREATE INDEX idx_consent_logs_purpose ON consent_logs(purpose);
CREATE INDEX idx_consent_logs_consented_at ON consent_logs(consented_at);
