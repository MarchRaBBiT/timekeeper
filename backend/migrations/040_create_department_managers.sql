CREATE TABLE department_managers (
    department_id TEXT NOT NULL REFERENCES departments(id) ON DELETE CASCADE,
    user_id       TEXT NOT NULL REFERENCES users(id)       ON DELETE CASCADE,
    assigned_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (department_id, user_id)
);
CREATE INDEX idx_department_managers_user_id ON department_managers(user_id);
