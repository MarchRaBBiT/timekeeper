use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;

use crate::models::{
    leave_request::{LeaveRequest, LeaveType},
    overtime_request::OvertimeRequest,
};
use crate::types::{LeaveRequestId, OvertimeRequestId, UserId};

pub async fn insert_leave_request(
    pool: &PgPool,
    request: &LeaveRequest,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO leave_requests (id, user_id, leave_type, start_date, end_date, reason, status, \
         approved_by, approved_at, decision_comment, rejected_by, rejected_at, cancelled_at, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
    )
    .bind(request.id.to_string())
    .bind(request.user_id.to_string())
    .bind(request.leave_type.db_value())
    .bind(request.start_date)
    .bind(request.end_date)
    .bind(&request.reason)
    .bind(request.status.db_value())
    .bind(request.approved_by.map(|id| id.to_string()))
    .bind(request.approved_at)
    .bind(&request.decision_comment)
    .bind(request.rejected_by.map(|id| id.to_string()))
    .bind(request.rejected_at)
    .bind(request.cancelled_at)
    .bind(request.created_at)
    .bind(request.updated_at)
    .execute(pool)
    .await
    .map(|_| ())
}

pub async fn insert_overtime_request(
    pool: &PgPool,
    request: &OvertimeRequest,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO overtime_requests (id, user_id, date, planned_hours, reason, status, approved_by, \
         approved_at, decision_comment, rejected_by, rejected_at, cancelled_at, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
    )
    .bind(request.id.to_string())
    .bind(request.user_id.to_string())
    .bind(request.date)
    .bind(request.planned_hours)
    .bind(&request.reason)
    .bind(request.status.db_value())
    .bind(request.approved_by.map(|id| id.to_string()))
    .bind(request.approved_at)
    .bind(&request.decision_comment)
    .bind(request.rejected_by.map(|id| id.to_string()))
    .bind(request.rejected_at)
    .bind(request.cancelled_at)
    .bind(request.created_at)
    .bind(request.updated_at)
    .execute(pool)
    .await
    .map(|_| ())
}

pub async fn fetch_leave_request_by_id(
    pool: &PgPool,
    id: LeaveRequestId,
    user_id: UserId,
) -> Result<Option<LeaveRequest>, sqlx::Error> {
    sqlx::query_as::<_, LeaveRequest>(
        "SELECT id, user_id, leave_type, start_date, end_date, reason, status, approved_by, approved_at, \
         rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at \
         FROM leave_requests WHERE id = $1 AND user_id = $2",
    )
    .bind(id.to_string())
    .bind(user_id.to_string())
    .fetch_optional(pool)
    .await
}

pub async fn fetch_leave_request(
    pool: &PgPool,
    id: LeaveRequestId,
) -> Result<Option<LeaveRequest>, sqlx::Error> {
    sqlx::query_as::<_, LeaveRequest>(
        "SELECT id, user_id, leave_type, start_date, end_date, reason, status, approved_by, approved_at, \
         rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at \
         FROM leave_requests WHERE id = $1",
    )
    .bind(id.to_string())
    .fetch_optional(pool)
    .await
}

pub async fn fetch_overtime_request_by_id(
    pool: &PgPool,
    id: OvertimeRequestId,
    user_id: UserId,
) -> Result<Option<OvertimeRequest>, sqlx::Error> {
    sqlx::query_as::<_, OvertimeRequest>(
        "SELECT id, user_id, date, planned_hours, reason, status, approved_by, approved_at, rejected_by, rejected_at, \
         cancelled_at, decision_comment, created_at, updated_at \
         FROM overtime_requests WHERE id = $1 AND user_id = $2",
    )
    .bind(id.to_string())
    .bind(user_id.to_string())
    .fetch_optional(pool)
    .await
}

pub async fn fetch_overtime_request(
    pool: &PgPool,
    id: OvertimeRequestId,
) -> Result<Option<OvertimeRequest>, sqlx::Error> {
    sqlx::query_as::<_, OvertimeRequest>(
        "SELECT id, user_id, date, planned_hours, reason, status, approved_by, approved_at, rejected_by, rejected_at, \
         cancelled_at, decision_comment, created_at, updated_at \
         FROM overtime_requests WHERE id = $1",
    )
    .bind(id.to_string())
    .fetch_optional(pool)
    .await
}

pub async fn list_leave_requests_by_user(
    pool: &PgPool,
    user_id: UserId,
) -> Result<Vec<LeaveRequest>, sqlx::Error> {
    sqlx::query_as::<_, LeaveRequest>(
        "SELECT id, user_id, leave_type, start_date, end_date, reason, status, approved_by, approved_at, \
         rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at \
         FROM leave_requests WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(user_id.to_string())
    .fetch_all(pool)
    .await
}

pub async fn list_overtime_requests_by_user(
    pool: &PgPool,
    user_id: UserId,
) -> Result<Vec<OvertimeRequest>, sqlx::Error> {
    sqlx::query_as::<_, OvertimeRequest>(
        "SELECT id, user_id, date, planned_hours, reason, status, approved_by, approved_at, rejected_by, rejected_at, \
         cancelled_at, decision_comment, created_at, updated_at \
         FROM overtime_requests WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(user_id.to_string())
    .fetch_all(pool)
    .await
}

pub async fn update_leave_request(
    pool: &PgPool,
    id: LeaveRequestId,
    leave_type: LeaveType,
    start_date: NaiveDate,
    end_date: NaiveDate,
    reason: Option<String>,
    updated_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE leave_requests SET leave_type = $1, start_date = $2, end_date = $3, reason = $4, updated_at = $5 \
         WHERE id = $6",
    )
    .bind(leave_type.db_value())
    .bind(start_date)
    .bind(end_date)
    .bind(&reason)
    .bind(updated_at)
    .bind(id.to_string())
    .execute(pool)
    .await
    .map(|_| ())
}

pub async fn update_overtime_request(
    pool: &PgPool,
    id: OvertimeRequestId,
    date: NaiveDate,
    planned_hours: f64,
    reason: Option<String>,
    updated_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE overtime_requests SET date = $1, planned_hours = $2, reason = $3, updated_at = $4 WHERE id = $5",
    )
    .bind(date)
    .bind(planned_hours)
    .bind(&reason)
    .bind(updated_at)
    .bind(id.to_string())
    .execute(pool)
    .await
    .map(|_| ())
}

pub async fn cancel_leave_request(
    pool: &PgPool,
    id: LeaveRequestId,
    user_id: UserId,
    timestamp: DateTime<Utc>,
) -> Result<u64, sqlx::Error> {
    sqlx::query(
        "UPDATE leave_requests SET status = 'cancelled', cancelled_at = $1, updated_at = $2 \
         WHERE id = $3 AND user_id = $4 AND status = 'pending'",
    )
    .bind(timestamp)
    .bind(timestamp)
    .bind(id.to_string())
    .bind(user_id.to_string())
    .execute(pool)
    .await
    .map(|result| result.rows_affected())
}

pub async fn cancel_overtime_request(
    pool: &PgPool,
    id: OvertimeRequestId,
    user_id: UserId,
    timestamp: DateTime<Utc>,
) -> Result<u64, sqlx::Error> {
    sqlx::query(
        "UPDATE overtime_requests SET status = 'cancelled', cancelled_at = $1, updated_at = $2 \
         WHERE id = $3 AND user_id = $4 AND status = 'pending'",
    )
    .bind(timestamp)
    .bind(timestamp)
    .bind(id.to_string())
    .bind(user_id.to_string())
    .execute(pool)
    .await
    .map(|result| result.rows_affected())
}

pub async fn approve_leave_request(
    pool: &PgPool,
    request_id: LeaveRequestId,
    approver_id: UserId,
    comment: &str,
    timestamp: DateTime<Utc>,
) -> Result<u64, sqlx::Error> {
    sqlx::query(
        "UPDATE leave_requests SET status = 'approved', approved_by = $1, approved_at = $2, decision_comment = $3, updated_at = $4 \
         WHERE id = $5 AND status = 'pending'",
    )
    .bind(approver_id.to_string())
    .bind(timestamp)
    .bind(comment)
    .bind(timestamp)
    .bind(request_id.to_string())
    .execute(pool)
    .await
    .map(|result| result.rows_affected())
}

pub async fn approve_overtime_request(
    pool: &PgPool,
    request_id: OvertimeRequestId,
    approver_id: UserId,
    comment: &str,
    timestamp: DateTime<Utc>,
) -> Result<u64, sqlx::Error> {
    sqlx::query(
        "UPDATE overtime_requests SET status = 'approved', approved_by = $1, approved_at = $2, decision_comment = $3, updated_at = $4 \
         WHERE id = $5 AND status = 'pending'",
    )
    .bind(approver_id.to_string())
    .bind(timestamp)
    .bind(comment)
    .bind(timestamp)
    .bind(request_id.to_string())
    .execute(pool)
    .await
    .map(|result| result.rows_affected())
}

pub async fn reject_leave_request(
    pool: &PgPool,
    request_id: LeaveRequestId,
    approver_id: UserId,
    comment: &str,
    timestamp: DateTime<Utc>,
) -> Result<u64, sqlx::Error> {
    sqlx::query(
        "UPDATE leave_requests SET status = 'rejected', rejected_by = $1, rejected_at = $2, decision_comment = $3, updated_at = $4 \
         WHERE id = $5 AND status = 'pending'",
    )
    .bind(approver_id.to_string())
    .bind(timestamp)
    .bind(comment)
    .bind(timestamp)
    .bind(request_id.to_string())
    .execute(pool)
    .await
    .map(|result| result.rows_affected())
}

pub async fn reject_overtime_request(
    pool: &PgPool,
    request_id: OvertimeRequestId,
    approver_id: UserId,
    comment: &str,
    timestamp: DateTime<Utc>,
) -> Result<u64, sqlx::Error> {
    sqlx::query(
        "UPDATE overtime_requests SET status = 'rejected', rejected_by = $1, rejected_at = $2, decision_comment = $3, updated_at = $4 \
         WHERE id = $5 AND status = 'pending'",
    )
    .bind(approver_id.to_string())
    .bind(timestamp)
    .bind(comment)
    .bind(timestamp)
    .bind(request_id.to_string())
    .execute(pool)
    .await
    .map(|result| result.rows_affected())
}
