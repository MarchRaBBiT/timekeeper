ALTER TABLE resolved_workdays
    ADD COLUMN schedule_type TEXT NOT NULL DEFAULT 'fixed'
        CHECK (schedule_type IN ('fixed', 'flex'));

CREATE TABLE resolved_workday_core_time_windows (
    resolved_workday_id UUID NOT NULL REFERENCES resolved_workdays(id) ON DELETE CASCADE,
    weekday SMALLINT NOT NULL CHECK (weekday BETWEEN 1 AND 7),
    start_time TIME NOT NULL,
    start_day_offset SMALLINT NOT NULL DEFAULT 0 CHECK (start_day_offset = 0),
    end_time TIME NOT NULL,
    end_day_offset SMALLINT NOT NULL DEFAULT 0 CHECK (end_day_offset BETWEEN 0 AND 1),
    PRIMARY KEY (resolved_workday_id, weekday)
);

CREATE TRIGGER resolved_workday_core_time_windows_immutable_when_locked
BEFORE INSERT OR UPDATE OR DELETE ON resolved_workday_core_time_windows
FOR EACH ROW EXECUTE FUNCTION prevent_locked_resolved_workday_child_mutation();
