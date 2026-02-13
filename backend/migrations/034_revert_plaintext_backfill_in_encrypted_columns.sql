-- Revert migration 033 placeholder backfill.
-- Keep encrypted columns free from plaintext placeholder values.

UPDATE users
SET full_name_enc = NULL
WHERE full_name_enc = username;

UPDATE users
SET email_enc = NULL
WHERE email_enc = username || '@placeholder.local';

UPDATE archived_users
SET full_name_enc = NULL
WHERE full_name_enc = username;

UPDATE archived_users
SET email_enc = NULL
WHERE email_enc = username || '@placeholder.local';
