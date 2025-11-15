CREATE TABLE IF NOT EXISTS holiday_exceptions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    exception_date DATE NOT NULL,
    override BOOLEAN NOT NULL,
    reason TEXT,
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT holiday_exceptions_user_date_key UNIQUE (user_id, exception_date)
);

CREATE INDEX IF NOT EXISTS idx_holiday_exceptions_date
    ON holiday_exceptions (exception_date);

