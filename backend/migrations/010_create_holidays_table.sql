CREATE TABLE IF NOT EXISTS holidays (
    id UUID PRIMARY KEY,
    holiday_date DATE NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT holidays_holiday_date_key UNIQUE (holiday_date)
);

CREATE INDEX IF NOT EXISTS idx_holidays_date ON holidays (holiday_date);
