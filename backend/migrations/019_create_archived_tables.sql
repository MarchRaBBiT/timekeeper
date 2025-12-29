-- Migration: Create archived tables for soft-deleted users
-- This migration creates archive tables for users and their related data.
-- Session tokens (active_access_tokens, refresh_tokens) are NOT archived.

-- Archived users table
CREATE TABLE archived_users (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    full_name TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'employee',
    is_system_admin BOOLEAN NOT NULL DEFAULT FALSE,
    mfa_secret TEXT,
    mfa_enabled_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    archived_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    archived_by TEXT  -- ID of the admin who archived this user
);

CREATE INDEX idx_archived_users_archived_at ON archived_users(archived_at);
CREATE INDEX idx_archived_users_username ON archived_users(username);

-- Archived attendance records
CREATE TABLE archived_attendance (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    date DATE NOT NULL,
    clock_in_time TIMESTAMP WITHOUT TIME ZONE,
    clock_out_time TIMESTAMP WITHOUT TIME ZONE,
    status TEXT NOT NULL DEFAULT 'present',
    total_work_hours DOUBLE PRECISION,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    archived_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_archived_attendance_user_id ON archived_attendance(user_id);
CREATE INDEX idx_archived_attendance_date ON archived_attendance(date);

-- Archived break records
CREATE TABLE archived_break_records (
    id TEXT PRIMARY KEY,
    attendance_id TEXT NOT NULL,
    break_start_time TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    break_end_time TIMESTAMP WITHOUT TIME ZONE,
    duration_minutes INTEGER,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    archived_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_archived_break_records_attendance_id ON archived_break_records(attendance_id);

-- Archived leave requests
CREATE TABLE archived_leave_requests (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    leave_type TEXT NOT NULL,
    start_date DATE NOT NULL,
    end_date DATE NOT NULL,
    reason TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    approved_by TEXT,
    approved_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    archived_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_archived_leave_requests_user_id ON archived_leave_requests(user_id);

-- Archived overtime requests
CREATE TABLE archived_overtime_requests (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    date DATE NOT NULL,
    planned_hours DOUBLE PRECISION NOT NULL,
    reason TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    approved_by TEXT,
    approved_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    archived_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_archived_overtime_requests_user_id ON archived_overtime_requests(user_id);

-- Archived holiday exceptions
CREATE TABLE archived_holiday_exceptions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    exception_date DATE NOT NULL,
    override BOOLEAN NOT NULL,
    reason TEXT,
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    archived_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_archived_holiday_exceptions_user_id ON archived_holiday_exceptions(user_id);
