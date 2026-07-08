-- T-04 (docs/design-docs/leave-entitlement.md): append-only leave ledger.
-- Balance is always derived from these events; no balance snapshot table exists.
-- Rows are never updated or deleted; corrections are appended as reversing events.

CREATE TABLE leave_ledger_entries (
    id UUID PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    leave_type TEXT NOT NULL DEFAULT 'annual'
        CHECK (leave_type IN ('annual', 'compensatory')),
    kind TEXT NOT NULL
        CHECK (kind IN ('grant', 'consume', 'release', 'expire', 'adjust')),
    -- Grant lot identifier: minted by 'grant' (or a new-lot 'adjust'),
    -- referenced by consume/release/expire.
    lot_id UUID NOT NULL,
    -- Signed integer minutes: grant/release and positive adjust are > 0,
    -- consume/expire and negative adjust are < 0.
    amount_minutes INTEGER NOT NULL CHECK (amount_minutes <> 0),
    -- 1 day = N minutes conversion frozen per lot at grant time.
    day_equivalent_minutes INTEGER NOT NULL
        CHECK (day_equivalent_minutes BETWEEN 1 AND 1440),
    granted_at DATE,
    expires_at DATE,
    grant_base_date DATE,
    leave_request_id TEXT REFERENCES leave_requests(id) ON DELETE RESTRICT,
    reason TEXT CHECK (reason IS NULL OR char_length(reason) <= 500),
    created_by TEXT REFERENCES users(id) ON DELETE RESTRICT,
    -- Point-in-time aggregation key for balance derivation.
    effective_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT leave_ledger_amount_sign CHECK (
        (kind IN ('grant', 'release') AND amount_minutes > 0)
        OR (kind IN ('consume', 'expire') AND amount_minutes < 0)
        OR kind = 'adjust'
    ),
    CONSTRAINT leave_ledger_grant_lot_fields CHECK (
        kind NOT IN ('grant') OR (
            granted_at IS NOT NULL
            AND expires_at IS NOT NULL
            AND grant_base_date IS NOT NULL
        )
    ),
    CONSTRAINT leave_ledger_request_reference CHECK (
        kind NOT IN ('consume', 'release') OR leave_request_id IS NOT NULL
    ),
    CONSTRAINT leave_ledger_adjust_reason CHECK (
        kind <> 'adjust' OR (reason IS NOT NULL AND char_length(reason) > 0)
    ),
    CONSTRAINT leave_ledger_expiry_after_grant CHECK (
        granted_at IS NULL OR expires_at IS NULL OR expires_at > granted_at
    )
);

CREATE INDEX idx_leave_ledger_user_type
    ON leave_ledger_entries (user_id, leave_type, effective_at);
CREATE INDEX idx_leave_ledger_lot ON leave_ledger_entries (lot_id);
CREATE INDEX idx_leave_ledger_request
    ON leave_ledger_entries (leave_request_id)
    WHERE leave_request_id IS NOT NULL;

-- Idempotent expiry backfill: at most one 'expire' event per lot.
CREATE UNIQUE INDEX uq_leave_ledger_expire
    ON leave_ledger_entries (lot_id)
    WHERE kind = 'expire';

-- Idempotent grant runs: at most one 'grant' per user/type/base date.
CREATE UNIQUE INDEX uq_leave_ledger_grant_base
    ON leave_ledger_entries (user_id, leave_type, grant_base_date)
    WHERE kind = 'grant';

-- Append-only enforcement at the database level.
CREATE OR REPLACE FUNCTION reject_leave_ledger_mutation()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'leave_ledger_entries is append-only (kind=%, id=%)',
        OLD.kind, OLD.id
        USING ERRCODE = 'raise_exception';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER leave_ledger_entries_append_only
    BEFORE UPDATE OR DELETE ON leave_ledger_entries
    FOR EACH ROW
    EXECUTE FUNCTION reject_leave_ledger_mutation();
