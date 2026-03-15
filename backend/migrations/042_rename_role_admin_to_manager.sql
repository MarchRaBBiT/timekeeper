UPDATE users        SET role = 'manager' WHERE role = 'admin';
UPDATE archived_users SET role = 'manager' WHERE role = 'admin';
