CREATE TABLE IF NOT EXISTS weekly_holidays (
    id TEXT PRIMARY KEY,
    weekday SMALLINT NOT NULL CHECK (weekday BETWEEN 0 AND 6),
    starts_on DATE NOT NULL,
    ends_on DATE,
    enforced_from DATE NOT NULL,
    enforced_to DATE,
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_weekly_holidays_weekday
    ON weekly_holidays (weekday);

CREATE INDEX IF NOT EXISTS idx_weekly_holidays_effective_range
    ON weekly_holidays (enforced_from, enforced_to);

