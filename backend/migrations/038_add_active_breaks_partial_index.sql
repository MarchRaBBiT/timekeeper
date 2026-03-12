CREATE INDEX IF NOT EXISTS idx_break_records_active_breaks
ON break_records (break_start_time DESC)
WHERE break_end_time IS NULL;
