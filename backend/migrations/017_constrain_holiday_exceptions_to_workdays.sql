-- Ensure personal holiday exceptions always represent a working-day override.
UPDATE holiday_exceptions
SET override = FALSE
WHERE override IS DISTINCT FROM FALSE;

ALTER TABLE holiday_exceptions
    ALTER COLUMN override SET DEFAULT FALSE,
    ADD CONSTRAINT holiday_exceptions_override_forces_workday CHECK (override = FALSE);
