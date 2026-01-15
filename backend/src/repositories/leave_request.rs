//! Leave request repository.
//!
//! Provides CRUD operations for leave requests.

use crate::error::AppError;
use crate::models::leave_request::{LeaveRequest, RequestStatus};
use crate::repositories::repository::Repository;
use crate::types::{LeaveRequestId, UserId};
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;

const TABLE_NAME: &str = "leave_requests";
const SELECT_COLUMNS: &str = "id, user_id, leave_type, start_date, end_date, reason, status, \
approved_by, approved_at, rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at";

#[derive(Debug, Default, Clone, Copy)]
pub struct LeaveRequestRepository;

impl LeaveRequestRepository {
    pub fn new() -> Self {
        Self
    }

    pub async fn find_by_user(
        &self,
        db: &PgPool,
        user_id: UserId,
    ) -> Result<Vec<LeaveRequest>, AppError> {
        let query = format!(
            "SELECT {} FROM {} WHERE user_id = $1 ORDER BY created_at DESC",
            SELECT_COLUMNS, TABLE_NAME
        );
        let rows = sqlx::query_as::<_, LeaveRequest>(&query)
            .bind(user_id)
            .fetch_all(db)
            .await?;
        Ok(rows)
    }

    #[allow(dead_code)]
    pub async fn find_by_user_and_date_range(
        &self,
        db: &PgPool,
        user_id: UserId,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<LeaveRequest>, AppError> {
        let query = format!(
            "SELECT {} FROM {} WHERE user_id = $1 AND start_date >= $2 AND end_date <= $3 \
             ORDER BY start_date DESC",
            SELECT_COLUMNS, TABLE_NAME
        );
        let rows = sqlx::query_as::<_, LeaveRequest>(&query)
            .bind(user_id)
            .bind(start_date)
            .bind(end_date)
            .fetch_all(db)
            .await?;
        Ok(rows)
    }

    pub async fn find_by_id_for_user(
        &self,
        db: &PgPool,
        id: LeaveRequestId,
        user_id: UserId,
    ) -> Result<Option<LeaveRequest>, AppError> {
        let query = format!(
            "SELECT {} FROM {} WHERE id = $1 AND user_id = $2",
            SELECT_COLUMNS, TABLE_NAME
        );
        let row = sqlx::query_as::<_, LeaveRequest>(&query)
            .bind(id)
            .bind(user_id)
            .fetch_optional(db)
            .await?;
        Ok(row)
    }

    pub async fn cancel(
        &self,
        db: &PgPool,
        id: LeaveRequestId,
        user_id: UserId,
        timestamp: DateTime<Utc>,
    ) -> Result<u64, AppError> {
        let query = format!(
            "UPDATE {} SET status = $1, cancelled_at = $2, updated_at = $3 \
             WHERE id = $4 AND user_id = $5 AND status = 'pending'",
            TABLE_NAME
        );
        let result = sqlx::query(&query)
            .bind(RequestStatus::Cancelled.db_value())
            .bind(timestamp)
            .bind(timestamp)
            .bind(id)
            .bind(user_id)
            .execute(db)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn approve(
        &self,
        db: &PgPool,
        id: LeaveRequestId,
        approver_id: UserId,
        comment: &str,
        timestamp: DateTime<Utc>,
    ) -> Result<u64, AppError> {
        let query = format!(
            "UPDATE {} SET status = $1, approved_by = $2, approved_at = $3, decision_comment = $4, \
             updated_at = $5 WHERE id = $6 AND status = 'pending'",
            TABLE_NAME
        );
        let result = sqlx::query(&query)
            .bind(RequestStatus::Approved.db_value())
            .bind(approver_id)
            .bind(timestamp)
            .bind(comment)
            .bind(timestamp)
            .bind(id)
            .execute(db)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn reject(
        &self,
        db: &PgPool,
        id: LeaveRequestId,
        approver_id: UserId,
        comment: &str,
        timestamp: DateTime<Utc>,
    ) -> Result<u64, AppError> {
        let query = format!(
            "UPDATE {} SET status = $1, rejected_by = $2, rejected_at = $3, decision_comment = $4, \
             updated_at = $5 WHERE id = $6 AND status = 'pending'",
            TABLE_NAME
        );
        let result = sqlx::query(&query)
            .bind(RequestStatus::Rejected.db_value())
            .bind(approver_id)
            .bind(timestamp)
            .bind(comment)
            .bind(timestamp)
            .bind(id)
            .execute(db)
            .await?;
        Ok(result.rows_affected())
    }

    fn base_select_query() -> String {
        format!("SELECT {} FROM {}", SELECT_COLUMNS, TABLE_NAME)
    }
}

impl Repository<LeaveRequest> for LeaveRequestRepository {
    const TABLE: &'static str = TABLE_NAME;
    type Id = LeaveRequestId;

    async fn find_all(&self, db: &PgPool) -> Result<Vec<LeaveRequest>, AppError> {
        let query = format!("{} ORDER BY start_date DESC", Self::base_select_query());
        let rows = sqlx::query_as::<_, LeaveRequest>(&query)
            .fetch_all(db)
            .await?;
        Ok(rows)
    }

    async fn find_by_id(&self, db: &PgPool, id: LeaveRequestId) -> Result<LeaveRequest, AppError> {
        let query = format!("{} WHERE id = $1", Self::base_select_query());
        let result = sqlx::query_as::<_, LeaveRequest>(&query)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Leave request not found".into()))?;
        Ok(result)
    }

    async fn create(&self, db: &PgPool, item: &LeaveRequest) -> Result<LeaveRequest, AppError> {
        let query = format!(
            "INSERT INTO {} (id, user_id, leave_type, start_date, end_date, reason, status, \
             approved_by, approved_at, decision_comment, rejected_by, rejected_at, cancelled_at, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15) \
             RETURNING {}",
            TABLE_NAME, SELECT_COLUMNS
        );
        let row = sqlx::query_as::<_, LeaveRequest>(&query)
            .bind(item.id)
            .bind(item.user_id)
            .bind(item.leave_type.db_value())
            .bind(item.start_date)
            .bind(item.end_date)
            .bind(&item.reason)
            .bind(item.status.db_value())
            .bind(item.approved_by)
            .bind(item.approved_at)
            .bind(&item.decision_comment)
            .bind(item.rejected_by)
            .bind(item.rejected_at)
            .bind(item.cancelled_at)
            .bind(item.created_at)
            .bind(item.updated_at)
            .fetch_one(db)
            .await?;
        Ok(row)
    }

    async fn update(&self, db: &PgPool, item: &LeaveRequest) -> Result<LeaveRequest, AppError> {
        let query = format!(
            "UPDATE {} SET user_id = $2, leave_type = $3, start_date = $4, end_date = $5, reason = $6, \
             status = $7, approved_by = $8, approved_at = $9, decision_comment = $10, rejected_by = $11, \
             rejected_at = $12, cancelled_at = $13, updated_at = $14 WHERE id = $1 \
             RETURNING {}",
            TABLE_NAME, SELECT_COLUMNS
        );
        let row = sqlx::query_as::<_, LeaveRequest>(&query)
            .bind(item.id)
            .bind(item.user_id)
            .bind(item.leave_type.db_value())
            .bind(item.start_date)
            .bind(item.end_date)
            .bind(&item.reason)
            .bind(item.status.db_value())
            .bind(item.approved_by)
            .bind(item.approved_at)
            .bind(&item.decision_comment)
            .bind(item.rejected_by)
            .bind(item.rejected_at)
            .bind(item.cancelled_at)
            .bind(item.updated_at)
            .fetch_one(db)
            .await?;
        Ok(row)
    }

    async fn delete(&self, db: &PgPool, id: LeaveRequestId) -> Result<(), AppError> {
        let query = format!("DELETE FROM {} WHERE id = $1", TABLE_NAME);
        sqlx::query(&query).bind(id).execute(db).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leave_request_select_columns_include_expected_fields() {
        assert!(SELECT_COLUMNS.contains("decision_comment"));
        assert!(SELECT_COLUMNS.contains("rejected_at"));
        assert!(SELECT_COLUMNS.contains("cancelled_at"));
    }
}
