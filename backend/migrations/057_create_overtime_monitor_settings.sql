CREATE TABLE overtime_monitor_settings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    valid_from DATE NOT NULL UNIQUE,
    fiscal_year_start_month SMALLINT NOT NULL CHECK (fiscal_year_start_month BETWEEN 1 AND 12),
    monthly_limit_minutes INTEGER NOT NULL CHECK (monthly_limit_minutes > 0),
    yearly_limit_minutes INTEGER NOT NULL CHECK (yearly_limit_minutes > 0),
    rolling_average_limit_minutes INTEGER NOT NULL CHECK (rolling_average_limit_minutes > 0),
    single_month_absolute_limit_minutes INTEGER NOT NULL CHECK (single_month_absolute_limit_minutes > 0),
    warning_ratio_percent SMALLINT NOT NULL CHECK (warning_ratio_percent BETWEEN 1 AND 100),
    overtime_request_tolerance_minutes INTEGER NOT NULL DEFAULT 0 CHECK (overtime_request_tolerance_minutes >= 0),
    break_six_hour_threshold_minutes INTEGER NOT NULL DEFAULT 360 CHECK (break_six_hour_threshold_minutes > 0),
    break_six_hour_minimum_minutes INTEGER NOT NULL DEFAULT 45 CHECK (break_six_hour_minimum_minutes >= 0),
    break_eight_hour_threshold_minutes INTEGER NOT NULL DEFAULT 480 CHECK (break_eight_hour_threshold_minutes > 0),
    break_eight_hour_minimum_minutes INTEGER NOT NULL DEFAULT 60 CHECK (break_eight_hour_minimum_minutes >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO overtime_monitor_settings (
    valid_from,
    fiscal_year_start_month,
    monthly_limit_minutes,
    yearly_limit_minutes,
    rolling_average_limit_minutes,
    single_month_absolute_limit_minutes,
    warning_ratio_percent,
    overtime_request_tolerance_minutes
) VALUES (
    '1900-01-01',
    4,
    2700,
    21600,
    4800,
    6000,
    80,
    0
);
