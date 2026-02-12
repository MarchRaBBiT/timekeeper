-- Backfill rows that predate encrypted columns and still have NULL in *_enc.
-- This keeps runtime queries stable after plaintext columns are dropped.
-- Note: values are placeholders in encrypted columns; rotation/backfill jobs can re-encrypt later.

UPDATE users
SET
    full_name_enc = COALESCE(full_name_enc, username),
    email_enc = COALESCE(email_enc, username || '@placeholder.local')
WHERE full_name_enc IS NULL
   OR email_enc IS NULL;

UPDATE archived_users
SET
    full_name_enc = COALESCE(full_name_enc, username),
    email_enc = COALESCE(email_enc, username || '@placeholder.local')
WHERE full_name_enc IS NULL
   OR email_enc IS NULL;
