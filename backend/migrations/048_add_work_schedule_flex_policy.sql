ALTER TABLE work_schedule_versions
    ADD COLUMN schedule_type TEXT NOT NULL DEFAULT 'fixed'
        CHECK (schedule_type IN ('fixed', 'flex'));

CREATE TABLE work_schedule_settlement_periods (
    version_id UUID PRIMARY KEY REFERENCES work_schedule_versions(id) ON DELETE CASCADE,
    unit TEXT NOT NULL CHECK (unit IN ('monthly')),
    contracted_minutes_per_period INTEGER NOT NULL
        CHECK (contracted_minutes_per_period > 0)
);

CREATE TABLE work_schedule_core_time_windows (
    version_id UUID NOT NULL REFERENCES work_schedule_versions(id) ON DELETE CASCADE,
    weekday SMALLINT NOT NULL CHECK (weekday BETWEEN 1 AND 7),
    start_time TIME NOT NULL,
    start_day_offset SMALLINT NOT NULL DEFAULT 0 CHECK (start_day_offset = 0),
    end_time TIME NOT NULL,
    end_day_offset SMALLINT NOT NULL DEFAULT 0 CHECK (end_day_offset BETWEEN 0 AND 1),
    PRIMARY KEY (version_id, weekday)
);

CREATE OR REPLACE FUNCTION prevent_published_work_schedule_flex_mutation()
RETURNS TRIGGER AS $$
DECLARE
    parent_status TEXT;
BEGIN
    SELECT status INTO parent_status FROM work_schedule_versions
        WHERE id = COALESCE(NEW.version_id, OLD.version_id);
    IF parent_status = 'published' THEN
        RAISE EXCEPTION 'published work schedule versions are immutable'
            USING ERRCODE = '23514';
    END IF;
    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER work_schedule_settlement_periods_immutable
BEFORE INSERT OR UPDATE OR DELETE ON work_schedule_settlement_periods
FOR EACH ROW EXECUTE FUNCTION prevent_published_work_schedule_flex_mutation();

CREATE TRIGGER work_schedule_core_time_windows_immutable
BEFORE INSERT OR UPDATE OR DELETE ON work_schedule_core_time_windows
FOR EACH ROW EXECUTE FUNCTION prevent_published_work_schedule_flex_mutation();
