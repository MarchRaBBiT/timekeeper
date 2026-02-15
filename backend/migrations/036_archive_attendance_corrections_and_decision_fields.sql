ALTER TABLE archived_leave_requests
    ADD COLUMN decision_comment TEXT,
    ADD COLUMN rejected_by TEXT,
    ADD COLUMN rejected_at TIMESTAMPTZ,
    ADD COLUMN cancelled_at TIMESTAMPTZ;

ALTER TABLE archived_overtime_requests
    ADD COLUMN decision_comment TEXT,
    ADD COLUMN rejected_by TEXT,
    ADD COLUMN rejected_at TIMESTAMPTZ,
    ADD COLUMN cancelled_at TIMESTAMPTZ;

CREATE TABLE archived_attendance_correction_requests (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    attendance_id TEXT NOT NULL,
    date DATE NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    reason TEXT NOT NULL,
    original_snapshot_json JSONB NOT NULL,
    proposed_values_json JSONB NOT NULL,
    decision_comment TEXT,
    approved_by TEXT,
    approved_at TIMESTAMPTZ,
    rejected_by TEXT,
    rejected_at TIMESTAMPTZ,
    cancelled_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    archived_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_archived_attendance_correction_requests_user_id
    ON archived_attendance_correction_requests(user_id);

CREATE TABLE archived_attendance_correction_effective_values (
    attendance_id TEXT PRIMARY KEY,
    source_request_id TEXT NOT NULL,
    clock_in_time_corrected TIMESTAMP WITHOUT TIME ZONE,
    clock_out_time_corrected TIMESTAMP WITHOUT TIME ZONE,
    break_records_corrected_json JSONB NOT NULL,
    applied_by TEXT,
    applied_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    archived_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

