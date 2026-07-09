CREATE TABLE monthly_closing_workflows (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    year INTEGER NOT NULL CHECK (year BETWEEN 1900 AND 9999),
    month INTEGER NOT NULL CHECK (month BETWEEN 1 AND 12),
    status TEXT NOT NULL CHECK (
        status IN ('open', 'self_confirmed', 'approved', 'closed', 'reopened')
    ),
    self_confirmed_by TEXT REFERENCES users(id) ON DELETE RESTRICT,
    self_confirmed_at TIMESTAMPTZ,
    approved_by TEXT REFERENCES users(id) ON DELETE RESTRICT,
    approved_at TIMESTAMPTZ,
    closed_by TEXT REFERENCES users(id) ON DELETE RESTRICT,
    closed_at TIMESTAMPTZ,
    reopened_by TEXT REFERENCES users(id) ON DELETE RESTRICT,
    reopened_at TIMESTAMPTZ,
    reason TEXT CHECK (reason IS NULL OR char_length(reason) <= 500),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT monthly_closing_workflows_user_month_key UNIQUE (user_id, year, month)
);

CREATE TABLE monthly_closing_workflow_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_id UUID NOT NULL REFERENCES monthly_closing_workflows(id) ON DELETE CASCADE,
    from_status TEXT,
    to_status TEXT NOT NULL,
    acted_by TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    reason TEXT CHECK (reason IS NULL OR char_length(reason) <= 500),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
