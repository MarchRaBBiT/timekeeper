-- Insert default admin user
-- Password: admin123 (hashed with argon2)
INSERT INTO users (id, username, password_hash, full_name, role, is_system_admin) VALUES (
    'admin-001',
    'admin',
    '$argon2id$v=19$m=4096,t=3,p=1$YWRtaW4$admin123',
    '�V�X�e���Ǘ���',
    'admin',
    TRUE
);
