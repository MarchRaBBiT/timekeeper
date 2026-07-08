-- T-04 (docs/design-docs/leave-entitlement.md): externalized leave grant rules.
-- Statutory values (grant day table, expiry months, 5-day obligation) are seed
-- data here, not hardcoded in application code.

CREATE TABLE leave_grant_rules (
    id UUID PRIMARY KEY,
    leave_type TEXT NOT NULL DEFAULT 'annual'
        CHECK (leave_type IN ('annual')),
    accrual_kind TEXT NOT NULL DEFAULT 'standard'
        CHECK (accrual_kind IN ('standard', 'proportional')),
    tenure_months INTEGER NOT NULL CHECK (tenure_months >= 0),
    weekly_working_days INTEGER
        CHECK (weekly_working_days IS NULL OR weekly_working_days BETWEEN 1 AND 7),
    granted_days INTEGER NOT NULL CHECK (granted_days > 0),
    expiry_months INTEGER NOT NULL DEFAULT 24 CHECK (expiry_months > 0),
    day_equivalent_minutes INTEGER NOT NULL DEFAULT 480
        CHECK (day_equivalent_minutes BETWEEN 1 AND 1440),
    effective_from DATE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT leave_grant_rules_proportional_days CHECK (
        (accrual_kind = 'standard' AND weekly_working_days IS NULL)
        OR (accrual_kind = 'proportional' AND weekly_working_days IS NOT NULL)
    )
);

CREATE UNIQUE INDEX uq_leave_grant_rules_rule ON leave_grant_rules (
    leave_type,
    accrual_kind,
    tenure_months,
    COALESCE(weekly_working_days, 0),
    effective_from
);

-- Annual 5-day obligation parameters (labor standards act art. 39-7 equivalent).
-- Kept as master data so operators can adjust thresholds without code changes.
CREATE TABLE leave_obligation_rules (
    id UUID PRIMARY KEY,
    leave_type TEXT NOT NULL DEFAULT 'annual'
        CHECK (leave_type IN ('annual')),
    minimum_granted_days INTEGER NOT NULL CHECK (minimum_granted_days > 0),
    required_days INTEGER NOT NULL CHECK (required_days > 0),
    window_months INTEGER NOT NULL CHECK (window_months > 0),
    warning_lead_days INTEGER NOT NULL DEFAULT 90 CHECK (warning_lead_days >= 0),
    effective_from DATE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_leave_obligation_rules UNIQUE (leave_type, effective_from)
);

-- Default seed: standard accrual table (6 months -> 10 days ... 78 months -> 20 days),
-- 2-year expiry, 1 day = 480 minutes. Operators may append rows with a newer
-- effective_from to change company rules.
INSERT INTO leave_grant_rules
    (id, leave_type, accrual_kind, tenure_months, granted_days, expiry_months,
     day_equivalent_minutes, effective_from)
VALUES
    (gen_random_uuid(), 'annual', 'standard', 6, 10, 24, 480, DATE '2000-01-01'),
    (gen_random_uuid(), 'annual', 'standard', 18, 11, 24, 480, DATE '2000-01-01'),
    (gen_random_uuid(), 'annual', 'standard', 30, 12, 24, 480, DATE '2000-01-01'),
    (gen_random_uuid(), 'annual', 'standard', 42, 14, 24, 480, DATE '2000-01-01'),
    (gen_random_uuid(), 'annual', 'standard', 54, 16, 24, 480, DATE '2000-01-01'),
    (gen_random_uuid(), 'annual', 'standard', 66, 18, 24, 480, DATE '2000-01-01'),
    (gen_random_uuid(), 'annual', 'standard', 78, 20, 24, 480, DATE '2000-01-01');

-- Default seed: >=10 granted days => 5 days must be taken within 12 months.
INSERT INTO leave_obligation_rules
    (id, leave_type, minimum_granted_days, required_days, window_months,
     warning_lead_days, effective_from)
VALUES
    (gen_random_uuid(), 'annual', 10, 5, 12, 90, DATE '2000-01-01');
