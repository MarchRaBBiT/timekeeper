CREATE TABLE attendance_correction_requests (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    attendance_id TEXT NOT NULL REFERENCES attendance(id) ON DELETE CASCADE,
    date DATE NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    reason TEXT NOT NULL,
    original_snapshot_json JSONB NOT NULL,
    proposed_values_json JSONB NOT NULL,
    decision_comment TEXT,
    approved_by TEXT REFERENCES users(id) ON DELETE SET NULL,
    approved_at TIMESTAMP WITH TIME ZONE,
    rejected_by TEXT REFERENCES users(id) ON DELETE SET NULL,
    rejected_at TIMESTAMP WITH TIME ZONE,
    cancelled_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE attendance_correction_effective_values (
    attendance_id TEXT PRIMARY KEY REFERENCES attendance(id) ON DELETE CASCADE,
    source_request_id TEXT NOT NULL REFERENCES attendance_correction_requests(id) ON DELETE CASCADE,
    clock_in_time_corrected TIMESTAMP WITHOUT TIME ZONE,
    clock_out_time_corrected TIMESTAMP WITHOUT TIME ZONE,
    break_records_corrected_json JSONB NOT NULL,
    applied_by TEXT REFERENCES users(id) ON DELETE SET NULL,
    applied_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_attendance_correction_requests_user_created
    ON attendance_correction_requests(user_id, created_at DESC);

CREATE INDEX idx_attendance_correction_requests_status_created
    ON attendance_correction_requests(status, created_at DESC);
