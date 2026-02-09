//! Overtime request repository trait for dependency injection and testing.
//!
//! This module defines the OvertimeRequestRepository trait which can be mocked
//! using mockall for testing purposes.

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;

use crate::error::AppError;
use crate::models::overtime_request::OvertimeRequest;
use crate::types::{OvertimeRequestId, UserId};

/// Repository trait for OvertimeRequest operations.
///
/// This trait is designed to be mockable using mockall for testing.
/// Use `MockOvertimeRequestRepository` in tests to mock the behavior.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
#[allow(dead_code)]
pub trait OvertimeRequestRepositoryTrait: Send + Sync {
    /// Find all overtime requests
    async fn find_all(&self, db: &PgPool) -> Result<Vec<OvertimeRequest>, AppError>;

    /// Find an overtime request by ID
    async fn find_by_id(
        &self,
        db: &PgPool,
        id: OvertimeRequestId,
    ) -> Result<OvertimeRequest, AppError>;

    /// Create a new overtime request
    async fn create(
        &self,
        db: &PgPool,
        item: &OvertimeRequest,
    ) -> Result<OvertimeRequest, AppError>;

    /// Update an existing overtime request
    async fn update(
        &self,
        db: &PgPool,
        item: &OvertimeRequest,
    ) -> Result<OvertimeRequest, AppError>;

    /// Delete an overtime request by ID
    async fn delete(&self, db: &PgPool, id: OvertimeRequestId) -> Result<(), AppError>;

    /// Find overtime requests by user
    async fn find_by_user(
        &self,
        db: &PgPool,
        user_id: UserId,
    ) -> Result<Vec<OvertimeRequest>, AppError>;

    /// Find overtime requests by user and date range
    async fn find_by_user_and_date_range(
        &self,
        db: &PgPool,
        user_id: UserId,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<OvertimeRequest>, AppError>;

    /// Find overtime request by ID for a specific user
    async fn find_by_id_for_user(
        &self,
        db: &PgPool,
        id: OvertimeRequestId,
        user_id: UserId,
    ) -> Result<Option<OvertimeRequest>, AppError>;

    /// Cancel an overtime request
    async fn cancel(
        &self,
        db: &PgPool,
        id: OvertimeRequestId,
        user_id: UserId,
        timestamp: DateTime<Utc>,
    ) -> Result<u64, AppError>;

    /// Approve an overtime request
    async fn approve(
        &self,
        db: &PgPool,
        id: OvertimeRequestId,
        approver_id: UserId,
        comment: &str,
        timestamp: DateTime<Utc>,
    ) -> Result<u64, AppError>;

    /// Reject an overtime request
    async fn reject(
        &self,
        db: &PgPool,
        id: OvertimeRequestId,
        approver_id: UserId,
        comment: &str,
        timestamp: DateTime<Utc>,
    ) -> Result<u64, AppError>;
}

/// Concrete implementation of OvertimeRequestRepositoryTrait
#[derive(Debug, Default, Clone, Copy)]
pub struct OvertimeRequestRepository;

impl OvertimeRequestRepository {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl OvertimeRequestRepositoryTrait for OvertimeRequestRepository {
    async fn find_all(&self, db: &PgPool) -> Result<Vec<OvertimeRequest>, AppError> {
        let query = format!(
            "SELECT {} FROM {} ORDER BY date DESC",
            "id, user_id, date, planned_hours, reason, status, approved_by, \
             approved_at, rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at",
            "overtime_requests"
        );
        let rows = sqlx::query_as::<_, OvertimeRequest>(&query)
            .fetch_all(db)
            .await?;
        Ok(rows)
    }

    async fn find_by_id(
        &self,
        db: &PgPool,
        id: OvertimeRequestId,
    ) -> Result<OvertimeRequest, AppError> {
        let query = format!(
            "SELECT {} FROM {} WHERE id = $1",
            "id, user_id, date, planned_hours, reason, status, approved_by, \
             approved_at, rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at",
            "overtime_requests"
        );
        let result = sqlx::query_as::<_, OvertimeRequest>(&query)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Overtime request not found".into()))?;
        Ok(result)
    }

    async fn create(
        &self,
        db: &PgPool,
        item: &OvertimeRequest,
    ) -> Result<OvertimeRequest, AppError> {
        use crate::models::request::RequestStatus;

        let query = format!(
            "INSERT INTO {} (id, user_id, date, planned_hours, reason, status, approved_by, approved_at, \
             decision_comment, rejected_by, rejected_at, cancelled_at, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) \
             RETURNING {}",
            "overtime_requests",
            "id, user_id, date, planned_hours, reason, status, approved_by, \
             approved_at, rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at"
        );
        let row = sqlx::query_as::<_, OvertimeRequest>(&query)
            .bind(item.id)
            .bind(item.user_id)
            .bind(item.date)
            .bind(item.planned_hours)
            .bind(&item.reason)
            .bind(match item.status {
                RequestStatus::Pending => "pending",
                RequestStatus::Approved => "approved",
                RequestStatus::Rejected => "rejected",
                RequestStatus::Cancelled => "cancelled",
            })
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

    async fn update(
        &self,
        db: &PgPool,
        item: &OvertimeRequest,
    ) -> Result<OvertimeRequest, AppError> {
        use crate::models::request::RequestStatus;

        let query = format!(
            "UPDATE {} SET user_id = $2, date = $3, planned_hours = $4, reason = $5, status = $6, \
             approved_by = $7, approved_at = $8, decision_comment = $9, rejected_by = $10, rejected_at = $11, \
             cancelled_at = $12, updated_at = $13 WHERE id = $1 RETURNING {}",
            "overtime_requests",
            "id, user_id, date, planned_hours, reason, status, approved_by, \
             approved_at, rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at"
        );
        let row = sqlx::query_as::<_, OvertimeRequest>(&query)
            .bind(item.id)
            .bind(item.user_id)
            .bind(item.date)
            .bind(item.planned_hours)
            .bind(&item.reason)
            .bind(match item.status {
                RequestStatus::Pending => "pending",
                RequestStatus::Approved => "approved",
                RequestStatus::Rejected => "rejected",
                RequestStatus::Cancelled => "cancelled",
            })
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

    async fn delete(&self, db: &PgPool, id: OvertimeRequestId) -> Result<(), AppError> {
        let query = format!("DELETE FROM {} WHERE id = $1", "overtime_requests");
        sqlx::query(&query).bind(id).execute(db).await?;
        Ok(())
    }

    async fn find_by_user(
        &self,
        db: &PgPool,
        user_id: UserId,
    ) -> Result<Vec<OvertimeRequest>, AppError> {
        let query = format!(
            "SELECT {} FROM {} WHERE user_id = $1 ORDER BY created_at DESC",
            "id, user_id, date, planned_hours, reason, status, approved_by, \
             approved_at, rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at",
            "overtime_requests"
        );
        let rows = sqlx::query_as::<_, OvertimeRequest>(&query)
            .bind(user_id)
            .fetch_all(db)
            .await?;
        Ok(rows)
    }

    async fn find_by_user_and_date_range(
        &self,
        db: &PgPool,
        user_id: UserId,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<OvertimeRequest>, AppError> {
        let query = format!(
            "SELECT {} FROM {} WHERE user_id = $1 AND date >= $2 AND date <= $3 \
             ORDER BY date DESC",
            "id, user_id, date, planned_hours, reason, status, approved_by, \
             approved_at, rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at",
            "overtime_requests"
        );
        let rows = sqlx::query_as::<_, OvertimeRequest>(&query)
            .bind(user_id)
            .bind(start_date)
            .bind(end_date)
            .fetch_all(db)
            .await?;
        Ok(rows)
    }

    async fn find_by_id_for_user(
        &self,
        db: &PgPool,
        id: OvertimeRequestId,
        user_id: UserId,
    ) -> Result<Option<OvertimeRequest>, AppError> {
        let query = format!(
            "SELECT {} FROM {} WHERE id = $1 AND user_id = $2",
            "id, user_id, date, planned_hours, reason, status, approved_by, \
             approved_at, rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at",
            "overtime_requests"
        );
        let row = sqlx::query_as::<_, OvertimeRequest>(&query)
            .bind(id)
            .bind(user_id)
            .fetch_optional(db)
            .await?;
        Ok(row)
    }

    async fn cancel(
        &self,
        db: &PgPool,
        id: OvertimeRequestId,
        user_id: UserId,
        timestamp: DateTime<Utc>,
    ) -> Result<u64, AppError> {
        use crate::models::request::RequestStatus;

        let query = format!(
            "UPDATE {} SET status = $1, cancelled_at = $2, updated_at = $3 \
             WHERE id = $4 AND user_id = $5 AND status = 'pending'",
            "overtime_requests"
        );
        let result = sqlx::query(&query)
            .bind(match RequestStatus::Cancelled {
                RequestStatus::Pending => "pending",
                RequestStatus::Approved => "approved",
                RequestStatus::Rejected => "rejected",
                RequestStatus::Cancelled => "cancelled",
            })
            .bind(timestamp)
            .bind(timestamp)
            .bind(id)
            .bind(user_id)
            .execute(db)
            .await?;
        Ok(result.rows_affected())
    }

    async fn approve(
        &self,
        db: &PgPool,
        id: OvertimeRequestId,
        approver_id: UserId,
        comment: &str,
        timestamp: DateTime<Utc>,
    ) -> Result<u64, AppError> {
        use crate::models::request::RequestStatus;

        let query = format!(
            "UPDATE {} SET status = $1, approved_by = $2, approved_at = $3, decision_comment = $4, \
             updated_at = $5 WHERE id = $6 AND status = 'pending'",
            "overtime_requests"
        );
        let result = sqlx::query(&query)
            .bind(match RequestStatus::Approved {
                RequestStatus::Pending => "pending",
                RequestStatus::Approved => "approved",
                RequestStatus::Rejected => "rejected",
                RequestStatus::Cancelled => "cancelled",
            })
            .bind(approver_id)
            .bind(timestamp)
            .bind(comment)
            .bind(timestamp)
            .bind(id)
            .execute(db)
            .await?;
        Ok(result.rows_affected())
    }

    async fn reject(
        &self,
        db: &PgPool,
        id: OvertimeRequestId,
        approver_id: UserId,
        comment: &str,
        timestamp: DateTime<Utc>,
    ) -> Result<u64, AppError> {
        use crate::models::request::RequestStatus;

        let query = format!(
            "UPDATE {} SET status = $1, rejected_by = $2, rejected_at = $3, decision_comment = $4, \
             updated_at = $5 WHERE id = $6 AND status = 'pending'",
            "overtime_requests"
        );
        let result = sqlx::query(&query)
            .bind(match RequestStatus::Rejected {
                RequestStatus::Pending => "pending",
                RequestStatus::Approved => "approved",
                RequestStatus::Rejected => "rejected",
                RequestStatus::Cancelled => "cancelled",
            })
            .bind(approver_id)
            .bind(timestamp)
            .bind(comment)
            .bind(timestamp)
            .bind(id)
            .execute(db)
            .await?;
        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_overtime_request_repository_can_be_created() {
        let _mock = MockOvertimeRequestRepositoryTrait::new();
    }

    #[test]
    fn test_mock_overtime_request_repository_trait_bounds() {
        fn check_send_sync<T: Send + Sync>() {}
        check_send_sync::<MockOvertimeRequestRepositoryTrait>();
    }

    #[test]
    fn overtime_request_repository_new_creates_instance() {
        let repo = OvertimeRequestRepository::new();
        let _repo = repo;
    }
}
