CREATE TABLE workday_overrides (
    id UUID PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    work_date DATE NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('non_working_day', 'use_schedule')),
    work_schedule_id UUID REFERENCES work_schedules(id) ON DELETE RESTRICT,
    reason TEXT NOT NULL,
    created_by TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT workday_overrides_user_date_key UNIQUE (user_id, work_date),
    CONSTRAINT workday_overrides_schedule_requirement CHECK (
        (kind = 'non_working_day' AND work_schedule_id IS NULL)
        OR (kind = 'use_schedule' AND work_schedule_id IS NOT NULL)
    ),
    CONSTRAINT workday_overrides_reason_length CHECK (char_length(reason) BETWEEN 1 AND 500)
);

CREATE INDEX workday_overrides_schedule_idx
    ON workday_overrides (work_schedule_id) WHERE work_schedule_id IS NOT NULL;

CREATE TABLE resolved_workdays (
    id UUID PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    work_date DATE NOT NULL,
    work_schedule_id UUID NOT NULL REFERENCES work_schedules(id) ON DELETE RESTRICT,
    work_schedule_version_id UUID NOT NULL
        REFERENCES work_schedule_versions(id) ON DELETE RESTRICT,
    source TEXT NOT NULL
        CHECK (source IN ('override', 'user', 'department', 'organization')),
    source_id UUID NOT NULL,
    day_kind TEXT NOT NULL
        CHECK (day_kind IN (
            'scheduled_workday',
            'scheduled_non_working_day',
            'public_holiday'
        )),
    timezone TEXT NOT NULL CHECK (char_length(timezone) BETWEEN 1 AND 100),
    workday_boundary TIME NOT NULL,
    expected_work_minutes INTEGER NOT NULL CHECK (expected_work_minutes >= 0),
    resolved_at TIMESTAMPTZ NOT NULL,
    locked_at TIMESTAMPTZ,
    CONSTRAINT resolved_workdays_user_date_key UNIQUE (user_id, work_date)
);

CREATE INDEX resolved_workdays_schedule_date_idx
    ON resolved_workdays (work_schedule_id, work_date);
CREATE INDEX resolved_workdays_version_idx
    ON resolved_workdays (work_schedule_version_id);

CREATE TABLE resolved_workday_intervals (
    resolved_workday_id UUID NOT NULL REFERENCES resolved_workdays(id) ON DELETE CASCADE,
    sequence INTEGER NOT NULL CHECK (sequence > 0),
    start_time TIME NOT NULL,
    start_day_offset SMALLINT NOT NULL CHECK (start_day_offset = 0),
    end_time TIME NOT NULL,
    end_day_offset SMALLINT NOT NULL CHECK (end_day_offset BETWEEN 0 AND 1),
    PRIMARY KEY (resolved_workday_id, sequence)
);

CREATE TABLE resolved_workday_breaks (
    resolved_workday_id UUID NOT NULL REFERENCES resolved_workdays(id) ON DELETE CASCADE,
    sequence INTEGER NOT NULL CHECK (sequence > 0),
    start_time TIME NOT NULL,
    start_day_offset SMALLINT NOT NULL CHECK (start_day_offset BETWEEN 0 AND 1),
    end_time TIME NOT NULL,
    end_day_offset SMALLINT NOT NULL CHECK (end_day_offset BETWEEN 0 AND 1),
    PRIMARY KEY (resolved_workday_id, sequence)
);

CREATE OR REPLACE FUNCTION prevent_locked_resolved_workday_mutation()
RETURNS TRIGGER AS $$
BEGIN
    IF OLD.locked_at IS NOT NULL THEN
        RAISE EXCEPTION 'locked resolved workdays are immutable'
            USING ERRCODE = '23514';
    END IF;
    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER resolved_workdays_immutable_when_locked
BEFORE UPDATE OR DELETE ON resolved_workdays
FOR EACH ROW EXECUTE FUNCTION prevent_locked_resolved_workday_mutation();

CREATE OR REPLACE FUNCTION prevent_locked_resolved_workday_child_mutation()
RETURNS TRIGGER AS $$
DECLARE
    parent_id UUID;
    parent_locked_at TIMESTAMPTZ;
BEGIN
    parent_id := CASE
        WHEN TG_OP = 'DELETE' THEN OLD.resolved_workday_id
        ELSE NEW.resolved_workday_id
    END;
    SELECT locked_at INTO parent_locked_at FROM resolved_workdays WHERE id = parent_id;
    IF parent_locked_at IS NOT NULL THEN
        RAISE EXCEPTION 'children of locked resolved workdays are immutable'
            USING ERRCODE = '23514';
    END IF;
    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER resolved_workday_intervals_immutable_when_locked
BEFORE INSERT OR UPDATE OR DELETE ON resolved_workday_intervals
FOR EACH ROW EXECUTE FUNCTION prevent_locked_resolved_workday_child_mutation();

CREATE TRIGGER resolved_workday_breaks_immutable_when_locked
BEFORE INSERT OR UPDATE OR DELETE ON resolved_workday_breaks
FOR EACH ROW EXECUTE FUNCTION prevent_locked_resolved_workday_child_mutation();
