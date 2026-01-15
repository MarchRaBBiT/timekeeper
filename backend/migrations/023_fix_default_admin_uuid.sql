-- Ensure default admin uses a UUIDv5 derived from "admin-001".
DO $$
DECLARE
    old_id TEXT := 'admin-001';
    new_id TEXT := '05d512df-1c7b-5567-9cbc-efa7104e9a90';
BEGIN
    IF EXISTS (SELECT 1 FROM users WHERE id = old_id) THEN
        -- Avoid username uniqueness conflicts when inserting the new row.
        UPDATE users
        SET username = username || '_legacy_admin_001'
        WHERE id = old_id AND username = 'admin';

        IF NOT EXISTS (SELECT 1 FROM users WHERE id = new_id) THEN
            INSERT INTO users (
                id,
                username,
                password_hash,
                full_name,
                role,
                is_system_admin,
                mfa_secret,
                mfa_enabled_at,
                created_at,
                updated_at
            )
            SELECT
                new_id,
                'admin',
                password_hash,
                full_name,
                role,
                is_system_admin,
                mfa_secret,
                mfa_enabled_at,
                created_at,
                updated_at
            FROM users
            WHERE id = old_id;
        END IF;

        UPDATE attendance SET user_id = new_id WHERE user_id = old_id;
        UPDATE attendance SET updated_by = new_id WHERE updated_by = old_id;
        UPDATE break_records SET updated_by = new_id WHERE updated_by = old_id;

        UPDATE leave_requests SET user_id = new_id WHERE user_id = old_id;
        UPDATE leave_requests SET approved_by = new_id WHERE approved_by = old_id;
        UPDATE leave_requests SET rejected_by = new_id WHERE rejected_by = old_id;
        UPDATE leave_requests SET updated_by = new_id WHERE updated_by = old_id;

        UPDATE overtime_requests SET user_id = new_id WHERE user_id = old_id;
        UPDATE overtime_requests SET approved_by = new_id WHERE approved_by = old_id;
        UPDATE overtime_requests SET rejected_by = new_id WHERE rejected_by = old_id;
        UPDATE overtime_requests SET updated_by = new_id WHERE updated_by = old_id;

        UPDATE refresh_tokens SET user_id = new_id WHERE user_id = old_id;
        UPDATE active_access_tokens SET user_id = new_id WHERE user_id = old_id;

        UPDATE consent_logs SET user_id = new_id WHERE user_id = old_id;
        UPDATE user_permissions SET user_id = new_id WHERE user_id = old_id;

        UPDATE holiday_exceptions SET user_id = new_id WHERE user_id = old_id;
        UPDATE holiday_exceptions SET created_by = new_id WHERE created_by = old_id;

        UPDATE subject_requests SET user_id = new_id WHERE user_id = old_id;
        UPDATE subject_requests SET approved_by = new_id WHERE approved_by = old_id;
        UPDATE subject_requests SET rejected_by = new_id WHERE rejected_by = old_id;

        UPDATE audit_logs SET actor_id = new_id WHERE actor_id = old_id;
        UPDATE audit_logs SET target_id = new_id WHERE target_id = old_id;

        IF EXISTS (
            SELECT 1
            FROM information_schema.tables
            WHERE table_name = 'audit_logs_legacy'
        ) THEN
            UPDATE audit_logs_legacy SET user_id = new_id WHERE user_id = old_id;
        END IF;

        UPDATE archived_users SET archived_by = new_id WHERE archived_by = old_id;
        UPDATE archived_attendance SET user_id = new_id WHERE user_id = old_id;
        UPDATE archived_leave_requests SET user_id = new_id WHERE user_id = old_id;
        UPDATE archived_leave_requests SET approved_by = new_id WHERE approved_by = old_id;
        UPDATE archived_overtime_requests SET user_id = new_id WHERE user_id = old_id;
        UPDATE archived_overtime_requests SET approved_by = new_id WHERE approved_by = old_id;
        UPDATE archived_holiday_exceptions SET user_id = new_id WHERE user_id = old_id;
        UPDATE archived_holiday_exceptions SET created_by = new_id WHERE created_by = old_id;

        DELETE FROM users WHERE id = old_id;
    END IF;
END $$;
