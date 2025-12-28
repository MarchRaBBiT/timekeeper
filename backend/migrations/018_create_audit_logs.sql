-- Replace legacy audit logs table with the expanded audit log schema.
ALTER TABLE IF EXISTS audit_logs RENAME TO audit_logs_legacy;

CREATE TABLE audit_logs (
    id TEXT PRIMARY KEY,
    occurred_at TIMESTAMP WITH TIME ZONE NOT NULL,
    actor_id TEXT,
    actor_type TEXT NOT NULL,
    event_type TEXT NOT NULL,
    target_type TEXT,
    target_id TEXT,
    result TEXT NOT NULL,
    error_code TEXT,
    metadata JSONB,
    ip TEXT,
    user_agent TEXT,
    request_id TEXT
);

CREATE INDEX idx_audit_logs_occurred_at ON audit_logs(occurred_at);
CREATE INDEX idx_audit_logs_actor_id ON audit_logs(actor_id);
CREATE INDEX idx_audit_logs_event_type ON audit_logs(event_type);
CREATE INDEX idx_audit_logs_target_id ON audit_logs(target_id);
CREATE INDEX idx_audit_logs_request_id ON audit_logs(request_id);
