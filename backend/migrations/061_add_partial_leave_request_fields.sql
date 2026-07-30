ALTER TABLE leave_requests
    ADD COLUMN acquisition_unit TEXT NOT NULL DEFAULT 'day'
        CHECK (acquisition_unit IN ('day', 'half_am', 'half_pm', 'hour')),
    ADD COLUMN start_time TIME,
    ADD COLUMN end_time TIME,
    ADD COLUMN requested_minutes INTEGER CHECK (requested_minutes > 0),
    ADD CONSTRAINT leave_requests_partial_time_shape CHECK (
        (
            acquisition_unit = 'hour'
            AND start_date = end_date
            AND start_time IS NOT NULL
            AND end_time IS NOT NULL
            AND start_time < end_time
            AND requested_minutes IS NOT NULL
        )
        OR (
            acquisition_unit <> 'hour'
            AND start_time IS NULL
            AND end_time IS NULL
        )
    );

ALTER TABLE archived_leave_requests
    ADD COLUMN acquisition_unit TEXT NOT NULL DEFAULT 'day',
    ADD COLUMN start_time TIME,
    ADD COLUMN end_time TIME,
    ADD COLUMN requested_minutes INTEGER;

ALTER TABLE leave_ledger_entries
    ADD COLUMN obligation_minutes INTEGER NOT NULL DEFAULT 0;

UPDATE leave_ledger_entries
SET obligation_minutes = amount_minutes
WHERE kind IN ('consume', 'release');
