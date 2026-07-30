CREATE TABLE compensatory_leave_settings (
    id UUID PRIMARY KEY,
    effective_from DATE NOT NULL UNIQUE,
    expiry_months INTEGER NOT NULL CHECK (expiry_months BETWEEN 1 AND 120),
    created_by TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO leave_types (code, name, is_paid, balance_tracked, allowed_units)
VALUES ('compensatory', 'Compensatory leave', FALSE, TRUE, ARRAY['day', 'half_am', 'half_pm', 'hour'])
ON CONFLICT (code) DO NOTHING;

ALTER TABLE leave_ledger_entries
    ADD COLUMN holiday_work_request_id UUID
        REFERENCES holiday_work_requests(id) ON DELETE RESTRICT;

CREATE UNIQUE INDEX uq_leave_ledger_holiday_work_grant
    ON leave_ledger_entries (holiday_work_request_id)
    WHERE kind = 'grant' AND holiday_work_request_id IS NOT NULL;

CREATE INDEX idx_leave_ledger_holiday_work_source
    ON leave_ledger_entries (holiday_work_request_id)
    WHERE holiday_work_request_id IS NOT NULL;
