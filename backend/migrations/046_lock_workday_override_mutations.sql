CREATE OR REPLACE FUNCTION prevent_locked_resolved_workday_override_mutation()
RETURNS TRIGGER AS $$
DECLARE
    target_user_id TEXT;
    target_work_date DATE;
    parent_locked_at TIMESTAMPTZ;
BEGIN
    target_user_id := CASE
        WHEN TG_OP = 'DELETE' THEN OLD.user_id
        ELSE NEW.user_id
    END;
    target_work_date := CASE
        WHEN TG_OP = 'DELETE' THEN OLD.work_date
        ELSE NEW.work_date
    END;

    SELECT locked_at INTO parent_locked_at
    FROM resolved_workdays
    WHERE user_id = target_user_id AND work_date = target_work_date
    FOR UPDATE;

    IF parent_locked_at IS NOT NULL THEN
        RAISE EXCEPTION 'resolved workdays are locked'
            USING ERRCODE = '23514';
    END IF;

    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER workday_overrides_immutable_when_locked
BEFORE INSERT OR UPDATE OR DELETE ON workday_overrides
FOR EACH ROW EXECUTE FUNCTION prevent_locked_resolved_workday_override_mutation();
