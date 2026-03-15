ALTER TABLE users ADD COLUMN department_id TEXT REFERENCES departments(id) ON DELETE SET NULL;
CREATE INDEX idx_users_department_id ON users(department_id);
