-- Convert total_work_hours to double precision to match Rust f64 type.
ALTER TABLE attendance
    ALTER COLUMN total_work_hours TYPE DOUBLE PRECISION
    USING total_work_hours::double precision;
