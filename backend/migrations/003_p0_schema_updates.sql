-- P0 schema updates: requests enhancements, audit support, admin edits

-- Leave requests: add decision_comment, rejected_by/at, cancelled_at, updated_by
ALTER TABLE leave_requests ADD COLUMN decision_comment TEXT;
ALTER TABLE leave_requests ADD COLUMN rejected_by TEXT REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE leave_requests ADD COLUMN rejected_at TIMESTAMP WITH TIME ZONE;
ALTER TABLE leave_requests ADD COLUMN cancelled_at TIMESTAMP WITH TIME ZONE;
ALTER TABLE leave_requests ADD COLUMN updated_by TEXT REFERENCES users(id) ON DELETE SET NULL;

-- Overtime requests: add decision_comment, rejected_by/at, cancelled_at, updated_by
ALTER TABLE overtime_requests ADD COLUMN decision_comment TEXT;
ALTER TABLE overtime_requests ADD COLUMN rejected_by TEXT REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE overtime_requests ADD COLUMN rejected_at TIMESTAMP WITH TIME ZONE;
ALTER TABLE overtime_requests ADD COLUMN cancelled_at TIMESTAMP WITH TIME ZONE;
ALTER TABLE overtime_requests ADD COLUMN updated_by TEXT REFERENCES users(id) ON DELETE SET NULL;

-- Attendance: track updater
ALTER TABLE attendance ADD COLUMN updated_by TEXT REFERENCES users(id) ON DELETE SET NULL;

-- Break records: track updater
ALTER TABLE break_records ADD COLUMN updated_by TEXT REFERENCES users(id) ON DELETE SET NULL;

-- Simple audit logs table for admin operations (optional use)
CREATE TABLE IF NOT EXISTS audit_logs (
    id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL, -- e.g., 'attendance', 'leave_request', 'overtime_request', 'break_record'
    entity_id TEXT NOT NULL,
    action TEXT NOT NULL, -- e.g., 'create', 'update', 'force_end', 'cancel', 'approve', 'reject'
    before_json TEXT,
    after_json TEXT,
    user_id TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL
);

-- Helpful indexes
CREATE INDEX IF NOT EXISTS idx_leave_requests_cancelled_at ON leave_requests(cancelled_at);
CREATE INDEX IF NOT EXISTS idx_overtime_requests_cancelled_at ON overtime_requests(cancelled_at);
CREATE INDEX IF NOT EXISTS idx_audit_logs_entity ON audit_logs(entity_type, entity_id);

