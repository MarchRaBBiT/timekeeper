-- Normalize user role text to snake_case to match sqlx enum mapping
UPDATE users SET role = lower(role);

