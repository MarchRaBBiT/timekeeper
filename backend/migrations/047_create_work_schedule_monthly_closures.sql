CREATE TABLE work_schedule_monthly_closures (
    id UUID PRIMARY KEY,
    year INTEGER NOT NULL CHECK (year BETWEEN 1900 AND 9999),
    month INTEGER NOT NULL CHECK (month BETWEEN 1 AND 12),
    period_start DATE NOT NULL,
    period_end DATE NOT NULL,
    user_ids TEXT[] NOT NULL DEFAULT '{}',
    locked_count BIGINT NOT NULL CHECK (locked_count >= 0),
    closed_by TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    reason TEXT CHECK (reason IS NULL OR char_length(reason) <= 500),
    closed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT work_schedule_monthly_closures_period_check CHECK (period_end >= period_start)
);

CREATE INDEX work_schedule_monthly_closures_period_idx
    ON work_schedule_monthly_closures (period_start, period_end);
