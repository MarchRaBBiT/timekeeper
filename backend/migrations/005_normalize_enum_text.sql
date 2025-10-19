-- Normalize enum-like text values to snake_case to match sqlx mappings

-- Attendance statuses (e.g., 'Present' -> 'present')
UPDATE attendance SET status = lower(status);

-- Leave requests: leave_type and status to snake_case
UPDATE leave_requests SET leave_type = lower(leave_type), status = lower(status);

-- Overtime requests: status to snake_case
UPDATE overtime_requests SET status = lower(status);

