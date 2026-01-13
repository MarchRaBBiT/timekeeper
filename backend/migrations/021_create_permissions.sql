CREATE TABLE permissions (
    name TEXT PRIMARY KEY,
    description TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE user_permissions (
    user_id TEXT NOT NULL,
    permission_name TEXT NOT NULL,
    granted_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (user_id, permission_name),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (permission_name) REFERENCES permissions(name) ON DELETE CASCADE
);

CREATE INDEX idx_user_permissions_user_id ON user_permissions(user_id);
CREATE INDEX idx_user_permissions_permission_name ON user_permissions(permission_name);

INSERT INTO permissions (name, description)
VALUES ('audit_log_read', 'Audit log read access')
ON CONFLICT (name) DO NOTHING;
