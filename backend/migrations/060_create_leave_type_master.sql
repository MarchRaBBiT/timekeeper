CREATE TABLE leave_types (
    code TEXT PRIMARY KEY CHECK (code ~ '^[a-z][a-z0-9_]{0,49}$'),
    name TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 100),
    is_paid BOOLEAN NOT NULL,
    balance_tracked BOOLEAN NOT NULL,
    allowed_units TEXT[] NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT leave_types_allowed_units_not_empty
        CHECK (cardinality(allowed_units) > 0),
    CONSTRAINT leave_types_allowed_units_valid
        CHECK (allowed_units <@ ARRAY['day', 'half_am', 'half_pm', 'hour']::TEXT[])
);

INSERT INTO leave_types (code, name, is_paid, balance_tracked, allowed_units)
VALUES
    ('annual', 'Annual leave', TRUE, TRUE, ARRAY['day', 'half_am', 'half_pm', 'hour']),
    ('sick', 'Sick leave', FALSE, FALSE, ARRAY['day']),
    ('personal', 'Personal leave', FALSE, FALSE, ARRAY['day']),
    ('other', 'Other leave', FALSE, FALSE, ARRAY['day']);

ALTER TABLE leave_requests
    ADD CONSTRAINT leave_requests_leave_type_fkey
    FOREIGN KEY (leave_type) REFERENCES leave_types(code) ON DELETE RESTRICT;

ALTER TABLE leave_grant_rules DROP CONSTRAINT leave_grant_rules_leave_type_check;
ALTER TABLE leave_grant_rules
    ADD CONSTRAINT leave_grant_rules_leave_type_fkey
    FOREIGN KEY (leave_type) REFERENCES leave_types(code) ON DELETE RESTRICT;

ALTER TABLE leave_obligation_rules DROP CONSTRAINT leave_obligation_rules_leave_type_check;
ALTER TABLE leave_obligation_rules
    ADD CONSTRAINT leave_obligation_rules_leave_type_fkey
    FOREIGN KEY (leave_type) REFERENCES leave_types(code) ON DELETE RESTRICT;

ALTER TABLE leave_ledger_entries DROP CONSTRAINT leave_ledger_entries_leave_type_check;
ALTER TABLE leave_ledger_entries
    ADD CONSTRAINT leave_ledger_entries_leave_type_fkey
    FOREIGN KEY (leave_type) REFERENCES leave_types(code) ON DELETE RESTRICT;
