CREATE TABLE holiday_work_requests (
    id UUID PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    work_date DATE NOT NULL,
    benefit TEXT NOT NULL CHECK (benefit IN ('substitution', 'compensatory')),
    substitute_date DATE,
    compensatory_minutes INTEGER CHECK (compensatory_minutes BETWEEN 1 AND 1440),
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'approved', 'rejected', 'cancelled')),
    reason TEXT NOT NULL CHECK (char_length(reason) BETWEEN 1 AND 500),
    decision_comment TEXT CHECK (char_length(decision_comment) <= 500),
    decided_by TEXT REFERENCES users(id) ON DELETE RESTRICT,
    decided_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT holiday_work_benefit_fields CHECK (
        (benefit = 'substitution' AND substitute_date IS NOT NULL
            AND substitute_date <> work_date AND compensatory_minutes IS NULL)
        OR
        (benefit = 'compensatory' AND substitute_date IS NULL
            AND compensatory_minutes IS NOT NULL)
    ),
    CONSTRAINT holiday_work_decision_fields CHECK (
        (status = 'pending' AND decided_by IS NULL AND decided_at IS NULL)
        OR (status = 'cancelled' AND decided_by IS NULL)
        OR (status IN ('approved', 'rejected') AND decided_by IS NOT NULL
            AND decided_at IS NOT NULL)
    )
);

CREATE UNIQUE INDEX uq_holiday_work_active_user_date
    ON holiday_work_requests (user_id, work_date)
    WHERE status IN ('pending', 'approved');
CREATE INDEX idx_holiday_work_user_created
    ON holiday_work_requests (user_id, created_at DESC);
CREATE INDEX idx_holiday_work_status_created
    ON holiday_work_requests (status, created_at DESC);
