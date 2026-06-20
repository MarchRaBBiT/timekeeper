CREATE EXTENSION IF NOT EXISTS btree_gist;

CREATE TABLE work_schedules (
    id UUID PRIMARY KEY,
    code TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'retired')),
    created_by TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT work_schedules_code_length CHECK (char_length(code) BETWEEN 1 AND 50),
    CONSTRAINT work_schedules_name_length CHECK (char_length(name) BETWEEN 1 AND 100),
    CONSTRAINT work_schedules_description_length
        CHECK (description IS NULL OR char_length(description) <= 1000)
);

CREATE UNIQUE INDEX work_schedules_code_ci_key ON work_schedules (lower(code));
CREATE INDEX work_schedules_status_updated_idx ON work_schedules (status, updated_at DESC);

CREATE TABLE work_schedule_versions (
    id UUID PRIMARY KEY,
    work_schedule_id UUID NOT NULL REFERENCES work_schedules(id) ON DELETE RESTRICT,
    version_number INTEGER NOT NULL CHECK (version_number > 0),
    status TEXT NOT NULL DEFAULT 'draft'
        CHECK (status IN ('draft', 'published', 'cancelled')),
    effective_from DATE NOT NULL,
    effective_until DATE,
    timezone TEXT NOT NULL,
    workday_boundary TIME NOT NULL,
    public_holiday_policy TEXT NOT NULL
        CHECK (public_holiday_policy IN ('non_working', 'follow_weekly_pattern')),
    late_grace_minutes INTEGER NOT NULL DEFAULT 0
        CHECK (late_grace_minutes BETWEEN 0 AND 1440),
    early_leave_grace_minutes INTEGER NOT NULL DEFAULT 0
        CHECK (early_leave_grace_minutes BETWEEN 0 AND 1440),
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    published_by TEXT REFERENCES users(id) ON DELETE RESTRICT,
    published_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT work_schedule_versions_number_key
        UNIQUE (work_schedule_id, version_number),
    CONSTRAINT work_schedule_versions_effective_range
        CHECK (effective_until IS NULL OR effective_until > effective_from),
    CONSTRAINT work_schedule_versions_publish_fields CHECK (
        (status = 'published' AND published_by IS NOT NULL AND published_at IS NOT NULL)
        OR status <> 'published'
    ),
    CONSTRAINT work_schedule_versions_published_period_excl
        EXCLUDE USING gist (
            work_schedule_id WITH =,
            daterange(effective_from, effective_until, '[)') WITH &&
        ) WHERE (status = 'published')
);

CREATE INDEX work_schedule_versions_schedule_idx
    ON work_schedule_versions (work_schedule_id, version_number DESC);

CREATE TABLE work_schedule_day_rules (
    id UUID PRIMARY KEY,
    version_id UUID NOT NULL REFERENCES work_schedule_versions(id) ON DELETE CASCADE,
    weekday SMALLINT NOT NULL CHECK (weekday BETWEEN 1 AND 7),
    day_kind TEXT NOT NULL CHECK (day_kind IN ('working_day', 'non_working_day')),
    expected_work_minutes INTEGER NOT NULL CHECK (expected_work_minutes >= 0),
    UNIQUE (version_id, weekday)
);

CREATE TABLE work_schedule_work_intervals (
    day_rule_id UUID NOT NULL REFERENCES work_schedule_day_rules(id) ON DELETE CASCADE,
    sequence INTEGER NOT NULL CHECK (sequence > 0),
    start_time TIME NOT NULL,
    start_day_offset SMALLINT NOT NULL DEFAULT 0 CHECK (start_day_offset = 0),
    end_time TIME NOT NULL,
    end_day_offset SMALLINT NOT NULL DEFAULT 0 CHECK (end_day_offset BETWEEN 0 AND 1),
    PRIMARY KEY (day_rule_id, sequence)
);

CREATE TABLE work_schedule_planned_breaks (
    day_rule_id UUID NOT NULL REFERENCES work_schedule_day_rules(id) ON DELETE CASCADE,
    sequence INTEGER NOT NULL CHECK (sequence > 0),
    start_time TIME NOT NULL,
    start_day_offset SMALLINT NOT NULL DEFAULT 0 CHECK (start_day_offset BETWEEN 0 AND 1),
    end_time TIME NOT NULL,
    end_day_offset SMALLINT NOT NULL DEFAULT 0 CHECK (end_day_offset BETWEEN 0 AND 1),
    PRIMARY KEY (day_rule_id, sequence)
);

CREATE TABLE work_schedule_assignments (
    id UUID PRIMARY KEY,
    work_schedule_id UUID NOT NULL REFERENCES work_schedules(id) ON DELETE RESTRICT,
    user_id TEXT REFERENCES users(id) ON DELETE RESTRICT,
    department_id TEXT REFERENCES departments(id) ON DELETE RESTRICT,
    is_org_default BOOLEAN NOT NULL DEFAULT FALSE,
    target_key TEXT GENERATED ALWAYS AS (
        CASE
            WHEN is_org_default THEN 'organization'
            WHEN department_id IS NOT NULL THEN 'department:' || department_id
            ELSE 'user:' || user_id
        END
    ) STORED,
    valid_from DATE NOT NULL,
    valid_until DATE,
    created_by TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT work_schedule_assignments_one_target CHECK (
        (user_id IS NOT NULL)::INTEGER
        + (department_id IS NOT NULL)::INTEGER
        + is_org_default::INTEGER = 1
    ),
    CONSTRAINT work_schedule_assignments_effective_range
        CHECK (valid_until IS NULL OR valid_until > valid_from),
    CONSTRAINT work_schedule_assignments_target_period_excl
        EXCLUDE USING gist (
            target_key WITH =,
            daterange(valid_from, valid_until, '[)') WITH &&
        )
);

CREATE INDEX work_schedule_assignments_schedule_idx
    ON work_schedule_assignments (work_schedule_id, valid_from);
CREATE INDEX work_schedule_assignments_user_idx
    ON work_schedule_assignments (user_id, valid_from) WHERE user_id IS NOT NULL;
CREATE INDEX work_schedule_assignments_department_idx
    ON work_schedule_assignments (department_id, valid_from) WHERE department_id IS NOT NULL;

CREATE OR REPLACE FUNCTION prevent_published_work_schedule_version_mutation()
RETURNS TRIGGER AS $$
BEGIN
    IF OLD.status = 'published' THEN
        RAISE EXCEPTION 'published work schedule versions are immutable'
            USING ERRCODE = '23514';
    END IF;
    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER work_schedule_versions_immutable
BEFORE UPDATE OR DELETE ON work_schedule_versions
FOR EACH ROW EXECUTE FUNCTION prevent_published_work_schedule_version_mutation();
