CREATE TABLE payroll_export_snapshots (
    id UUID PRIMARY KEY,
    workflow_id UUID NOT NULL REFERENCES monthly_closing_workflows(id) ON DELETE RESTRICT,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    year INTEGER NOT NULL,
    month INTEGER NOT NULL CHECK (month BETWEEN 1 AND 12),
    revision INTEGER NOT NULL CHECK (revision > 0),
    worked_minutes BIGINT NOT NULL CHECK (worked_minutes >= 0),
    scheduled_minutes BIGINT NOT NULL CHECK (scheduled_minutes >= 0),
    statutory_within_minutes BIGINT NOT NULL CHECK (statutory_within_minutes >= 0),
    statutory_excess_minutes BIGINT NOT NULL CHECK (statutory_excess_minutes >= 0),
    legal_holiday_minutes BIGINT NOT NULL CHECK (legal_holiday_minutes >= 0),
    night_minutes BIGINT NOT NULL CHECK (night_minutes >= 0),
    absent_days INTEGER NOT NULL CHECK (absent_days >= 0),
    paid_leave_days INTEGER NOT NULL CHECK (paid_leave_days >= 0),
    paid_leave_half_days INTEGER NOT NULL CHECK (paid_leave_half_days >= 0),
    paid_leave_minutes BIGINT NOT NULL CHECK (paid_leave_minutes >= 0),
    holiday_work_minutes BIGINT NOT NULL CHECK (holiday_work_minutes >= 0),
    substitute_holiday_days INTEGER NOT NULL CHECK (substitute_holiday_days >= 0),
    compensatory_leave_minutes BIGINT NOT NULL CHECK (compensatory_leave_minutes >= 0),
    created_by TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT payroll_export_snapshots_user_period_revision_key
        UNIQUE (user_id, year, month, revision),
    CONSTRAINT payroll_export_snapshots_workflow_revision_key UNIQUE (workflow_id, revision)
);

CREATE INDEX payroll_export_snapshots_period_idx
    ON payroll_export_snapshots (year, month, user_id, revision DESC);

CREATE OR REPLACE FUNCTION prevent_payroll_export_snapshot_mutation()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'payroll export snapshots are immutable'
        USING ERRCODE = '23514';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER payroll_export_snapshots_immutable
BEFORE UPDATE OR DELETE ON payroll_export_snapshots
FOR EACH ROW EXECUTE FUNCTION prevent_payroll_export_snapshot_mutation();
