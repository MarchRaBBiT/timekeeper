-- Add explicit system administrator flag and promote existing admin user
ALTER TABLE users
    ADD COLUMN is_system_admin BOOLEAN NOT NULL DEFAULT FALSE;

-- Ensure the seeded admin account retains system-level privileges
UPDATE users
SET is_system_admin = TRUE
WHERE username = 'admin';
