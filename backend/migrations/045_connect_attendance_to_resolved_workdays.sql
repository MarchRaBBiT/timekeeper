ALTER TABLE resolved_workdays
    ADD CONSTRAINT resolved_workdays_identity_key UNIQUE (id, user_id, work_date);

ALTER TABLE attendance
    ADD COLUMN resolved_workday_id UUID,
    ADD COLUMN is_unscheduled_work BOOLEAN NOT NULL DEFAULT FALSE,
    ADD CONSTRAINT attendance_resolved_workday_identity_fk
        FOREIGN KEY (resolved_workday_id, user_id, date)
        REFERENCES resolved_workdays (id, user_id, work_date)
        ON DELETE RESTRICT,
    ADD CONSTRAINT attendance_unscheduled_requires_workday CHECK (
        NOT is_unscheduled_work OR resolved_workday_id IS NOT NULL
    );

CREATE INDEX attendance_resolved_workday_idx
    ON attendance (resolved_workday_id) WHERE resolved_workday_id IS NOT NULL;

CREATE OR REPLACE FUNCTION validate_attendance_resolved_workday()
RETURNS TRIGGER AS $$
DECLARE
    resolved_locked_at TIMESTAMPTZ;
    resolved_day_kind TEXT;
BEGIN
    IF NEW.resolved_workday_id IS NULL THEN
        NEW.is_unscheduled_work := FALSE;
        RETURN NEW;
    END IF;

    SELECT locked_at, day_kind
    INTO resolved_locked_at, resolved_day_kind
    FROM resolved_workdays
    WHERE id = NEW.resolved_workday_id
      AND user_id = NEW.user_id
      AND work_date = NEW.date;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'attendance resolved workday does not match user and date'
            USING ERRCODE = '23503';
    END IF;
    IF resolved_locked_at IS NULL THEN
        RAISE EXCEPTION 'attendance requires a locked resolved workday'
            USING ERRCODE = '23514';
    END IF;

    NEW.is_unscheduled_work := resolved_day_kind <> 'scheduled_workday';
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER attendance_validate_resolved_workday
BEFORE INSERT OR UPDATE OF resolved_workday_id, user_id, date ON attendance
FOR EACH ROW EXECUTE FUNCTION validate_attendance_resolved_workday();
