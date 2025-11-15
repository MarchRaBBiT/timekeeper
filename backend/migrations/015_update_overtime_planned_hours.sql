-- Ensure overtime planned_hours matches backend f64 expectations.
ALTER TABLE overtime_requests
    ALTER COLUMN planned_hours TYPE DOUBLE PRECISION
    USING planned_hours::DOUBLE PRECISION;
