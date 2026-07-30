ALTER TABLE overtime_monitor_settings
ADD COLUMN minimum_rest_minutes INTEGER NOT NULL DEFAULT 660
    CHECK (minimum_rest_minutes > 0);

CREATE INDEX idx_attendance_user_id_date_desc
    ON attendance (user_id, date DESC);
