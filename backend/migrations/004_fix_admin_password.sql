-- Ensure default admin exists with a valid Argon2 hash for password 'admin123'

-- 1) Update existing admin user's password hash if present
UPDATE users
SET password_hash = '$argon2id$v=19$m=19456,t=2,p=1$SHjkBewYpN0AqfNJtz4BCQ$Q6EKet0GGAKSbQ5hQFsXf+6WG+AX8Z91wi5hlimtYew',
    updated_at = CURRENT_TIMESTAMP
WHERE username = 'admin';

-- 2) Insert admin user if not exists
INSERT INTO users (id, username, password_hash, full_name, role, created_at, updated_at)
SELECT 'admin-001', 'admin', '$argon2id$v=19$m=19456,t=2,p=1$SHjkBewYpN0AqfNJtz4BCQ$Q6EKet0GGAKSbQ5hQFsXf+6WG+AX8Z91wi5hlimtYew', 'System Administrator', 'admin', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM users WHERE username = 'admin');

