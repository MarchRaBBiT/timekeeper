-- Remove plaintext PII columns after encrypted columns are fully populated.
ALTER TABLE users
    DROP COLUMN IF EXISTS full_name,
    DROP COLUMN IF EXISTS email,
    DROP COLUMN IF EXISTS mfa_secret;

ALTER TABLE archived_users
    DROP COLUMN IF EXISTS full_name,
    DROP COLUMN IF EXISTS email,
    DROP COLUMN IF EXISTS mfa_secret;
