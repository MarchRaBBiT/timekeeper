use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::models::{leave_request::LeaveRequest, overtime_request::OvertimeRequest};

pub async fn fetch_leave_request(
    pool: &PgPool,
    id: &str,
) -> Result<Option<LeaveRequest>, sqlx::Error> {
    sqlx::query_as::<_, LeaveRequest>(
        "SELECT id, user_id, leave_type, start_date, end_date, reason, status, approved_by, approved_at, \
         rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at \
         FROM leave_requests WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn fetch_overtime_request(
    pool: &PgPool,
    id: &str,
) -> Result<Option<OvertimeRequest>, sqlx::Error> {
    sqlx::query_as::<_, OvertimeRequest>(
        "SELECT id, user_id, date, planned_hours, reason, status, approved_by, approved_at, rejected_by, rejected_at, \
         cancelled_at, decision_comment, created_at, updated_at \
         FROM overtime_requests WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn approve_leave_request(
    pool: &PgPool,
    request_id: &str,
    approver_id: &str,
    comment: &str,
    timestamp: DateTime<Utc>,
) -> Result<u64, sqlx::Error> {
    sqlx::query(
        "UPDATE leave_requests SET status = 'approved', approved_by = $1, approved_at = $2, decision_comment = $3, updated_at = $4 \
         WHERE id = $5 AND status = 'pending'",
    )
    .bind(approver_id)
    .bind(&timestamp)
    .bind(comment)
    .bind(&timestamp)
    .bind(request_id)
    .execute(pool)
    .await
    .map(|result| result.rows_affected())
}

pub async fn approve_overtime_request(
    pool: &PgPool,
    request_id: &str,
    approver_id: &str,
    comment: &str,
    timestamp: DateTime<Utc>,
) -> Result<u64, sqlx::Error> {
    sqlx::query(
        "UPDATE overtime_requests SET status = 'approved', approved_by = $1, approved_at = $2, decision_comment = $3, updated_at = $4 \
         WHERE id = $5 AND status = 'pending'",
    )
    .bind(approver_id)
    .bind(&timestamp)
    .bind(comment)
    .bind(&timestamp)
    .bind(request_id)
    .execute(pool)
    .await
    .map(|result| result.rows_affected())
}

pub async fn reject_leave_request(
    pool: &PgPool,
    request_id: &str,
    approver_id: &str,
    comment: &str,
    timestamp: DateTime<Utc>,
) -> Result<u64, sqlx::Error> {
    sqlx::query(
        "UPDATE leave_requests SET status = 'rejected', rejected_by = $1, rejected_at = $2, decision_comment = $3, updated_at = $4 \
         WHERE id = $5 AND status = 'pending'",
    )
    .bind(approver_id)
    .bind(&timestamp)
    .bind(comment)
    .bind(&timestamp)
    .bind(request_id)
    .execute(pool)
    .await
    .map(|result| result.rows_affected())
}

pub async fn reject_overtime_request(
    pool: &PgPool,
    request_id: &str,
    approver_id: &str,
    comment: &str,
    timestamp: DateTime<Utc>,
) -> Result<u64, sqlx::Error> {
    sqlx::query(
        "UPDATE overtime_requests SET status = 'rejected', rejected_by = $1, rejected_at = $2, decision_comment = $3, updated_at = $4 \
         WHERE id = $5 AND status = 'pending'",
    )
    .bind(approver_id)
    .bind(&timestamp)
    .bind(comment)
    .bind(&timestamp)
    .bind(request_id)
    .execute(pool)
    .await
    .map(|result| result.rows_affected())
}
