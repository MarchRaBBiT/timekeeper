//! Leave request repository trait for dependency injection and testing.
//!
//! This module defines the LeaveRequestRepository trait which can be mocked
//! using mockall for testing purposes.

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;

use crate::error::AppError;
use crate::models::leave_request::LeaveRequest;
use crate::types::{LeaveRequestId, UserId};

/// Repository trait for LeaveRequest operations.
///
/// This trait is designed to be mockable using mockall for testing.
/// Use `MockLeaveRequestRepository` in tests to mock the behavior.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
#[allow(dead_code)]
pub trait LeaveRequestRepositoryTrait: Send + Sync {
    /// Find all leave requests
    async fn find_all(&self, db: &PgPool) -> Result<Vec<LeaveRequest>, AppError>;

    /// Find a leave request by ID
    async fn find_by_id(&self, db: &PgPool, id: LeaveRequestId) -> Result<LeaveRequest, AppError>;

    /// Create a new leave request
    async fn create(&self, db: &PgPool, item: &LeaveRequest) -> Result<LeaveRequest, AppError>;

    /// Update an existing leave request
    async fn update(&self, db: &PgPool, item: &LeaveRequest) -> Result<LeaveRequest, AppError>;

    /// Delete a leave request by ID
    async fn delete(&self, db: &PgPool, id: LeaveRequestId) -> Result<(), AppError>;

    /// Find leave requests by user
    async fn find_by_user(&self, db: &PgPool, user_id: UserId) -> Result<Vec<LeaveRequest>, AppError>;

    /// Find leave requests by user and date range
    async fn find_by_user_and_date_range(
        &self,
        db: &PgPool,
        user_id: UserId,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<LeaveRequest>, AppError>;

    /// Find leave request by ID for a specific user
    async fn find_by_id_for_user(
        &self,
        db: &PgPool,
        id: LeaveRequestId,
        user_id: UserId,
    ) -> Result<Option<LeaveRequest>, AppError>;

    /// Cancel a leave request
    async fn cancel(
        &self,
        db: &PgPool,
        id: LeaveRequestId,
        user_id: UserId,
        timestamp: DateTime<Utc>,
    ) -> Result<u64, AppError>;

    /// Approve a leave request
    async fn approve(
        &self,
        db: &PgPool,
        id: LeaveRequestId,
        approver_id: UserId,
        comment: &str,
        timestamp: DateTime<Utc>,
    ) -> Result<u64, AppError>;

    /// Reject a leave request
    async fn reject(
        &self,
        db: &PgPool,
        id: LeaveRequestId,
        approver_id: UserId,
        comment: &str,
        timestamp: DateTime<Utc>,
    ) -> Result<u64, AppError>;
}

/// Concrete implementation of LeaveRequestRepositoryTrait
#[derive(Debug, Default, Clone, Copy)]
pub struct LeaveRequestRepository;

impl LeaveRequestRepository {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl LeaveRequestRepositoryTrait for LeaveRequestRepository {
    async fn find_all(&self, db: &PgPool) -> Result<Vec<LeaveRequest>, AppError> {
        let query = format!(
            "SELECT {} FROM {} ORDER BY start_date DESC",
            "id, user_id, leave_type, start_date, end_date, reason, status, \
             approved_by, approved_at, rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at",
            "leave_requests"
        );
        let rows = sqlx::query_as::<_, LeaveRequest>(&query)
            .fetch_all(db)
            .await?;
        Ok(rows)
    }

    async fn find_by_id(&self, db: &PgPool, id: LeaveRequestId) -> Result<LeaveRequest, AppError> {
        let query = format!(
            "SELECT {} FROM {} WHERE id = $1",
            "id, user_id, leave_type, start_date, end_date, reason, status, \
             approved_by, approved_at, rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at",
            "leave_requests"
        );
        let result = sqlx::query_as::<_, LeaveRequest>(&query)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Leave request not found".into()))?;
        Ok(result)
    }

    async fn create(&self, db: &PgPool, item: &LeaveRequest) -> Result<LeaveRequest, AppError> {
        use crate::models::leave_request::LeaveType;
        use crate::models::request::RequestStatus;

        let query = format!(
            "INSERT INTO {} (id, user_id, leave_type, start_date, end_date, reason, status, \
             approved_by, approved_at, decision_comment, rejected_by, rejected_at, cancelled_at, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15) \
             RETURNING {}",
            "leave_requests",
            "id, user_id, leave_type, start_date, end_date, reason, status, \
             approved_by, approved_at, rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at"
        );
        let row = sqlx::query_as::<_, LeaveRequest>(&query)
            .bind(item.id)
            .bind(item.user_id)
            .bind(match item.leave_type {
                LeaveType::Annual => "annual",
                LeaveType::Sick => "sick",
                LeaveType::Personal => "personal",
                LeaveType::Other => "other",
            })
            .bind(item.start_date)
            .bind(item.end_date)
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

    async fn update(&self, db: &PgPool, item: &LeaveRequest) -> Result<LeaveRequest, AppError> {
        use crate::models::leave_request::LeaveType;
        use crate::models::request::RequestStatus;

        let query = format!(
            "UPDATE {} SET user_id = $2, leave_type = $3, start_date = $4, end_date = $5, reason = $6, \
             status = $7, approved_by = $8, approved_at = $9, decision_comment = $10, rejected_by = $11, \
             rejected_at = $12, cancelled_at = $13, updated_at = $14 WHERE id = $1 \
             RETURNING {}",
            "leave_requests",
            "id, user_id, leave_type, start_date, end_date, reason, status, \
             approved_by, approved_at, rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at"
        );
        let row = sqlx::query_as::<_, LeaveRequest>(&query)
            .bind(item.id)
            .bind(item.user_id)
            .bind(match item.leave_type {
                LeaveType::Annual => "annual",
                LeaveType::Sick => "sick",
                LeaveType::Personal => "personal",
                LeaveType::Other => "other",
            })
            .bind(item.start_date)
            .bind(item.end_date)
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

    async fn delete(&self, db: &PgPool, id: LeaveRequestId) -> Result<(), AppError> {
        let query = format!("DELETE FROM {} WHERE id = $1", "leave_requests");
        sqlx::query(&query).bind(id).execute(db).await?;
        Ok(())
    }

    async fn find_by_user(&self, db: &PgPool, user_id: UserId) -> Result<Vec<LeaveRequest>, AppError> {
        let query = format!(
            "SELECT {} FROM {} WHERE user_id = $1 ORDER BY created_at DESC",
            "id, user_id, leave_type, start_date, end_date, reason, status, \
             approved_by, approved_at, rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at",
            "leave_requests"
        );
        let rows = sqlx::query_as::<_, LeaveRequest>(&query)
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
    ) -> Result<Vec<LeaveRequest>, AppError> {
        let query = format!(
            "SELECT {} FROM {} WHERE user_id = $1 AND start_date >= $2 AND end_date <= $3 \
             ORDER BY start_date DESC",
            "id, user_id, leave_type, start_date, end_date, reason, status, \
             approved_by, approved_at, rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at",
            "leave_requests"
        );
        let rows = sqlx::query_as::<_, LeaveRequest>(&query)
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
        id: LeaveRequestId,
        user_id: UserId,
    ) -> Result<Option<LeaveRequest>, AppError> {
        let query = format!(
            "SELECT {} FROM {} WHERE id = $1 AND user_id = $2",
            "id, user_id, leave_type, start_date, end_date, reason, status, \
             approved_by, approved_at, rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at",
            "leave_requests"
        );
        let row = sqlx::query_as::<_, LeaveRequest>(&query)
            .bind(id)
            .bind(user_id)
            .fetch_optional(db)
            .await?;
        Ok(row)
    }

    async fn cancel(
        &self,
        db: &PgPool,
        id: LeaveRequestId,
        user_id: UserId,
        timestamp: DateTime<Utc>,
    ) -> Result<u64, AppError> {
        use crate::models::request::RequestStatus;

        let query = format!(
            "UPDATE {} SET status = $1, cancelled_at = $2, updated_at = $3 \
             WHERE id = $4 AND user_id = $5 AND status = 'pending'",
            "leave_requests"
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
        id: LeaveRequestId,
        approver_id: UserId,
        comment: &str,
        timestamp: DateTime<Utc>,
    ) -> Result<u64, AppError> {
        use crate::models::request::RequestStatus;

        let query = format!(
            "UPDATE {} SET status = $1, approved_by = $2, approved_at = $3, decision_comment = $4, \
             updated_at = $5 WHERE id = $6 AND status = 'pending'",
            "leave_requests"
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
        id: LeaveRequestId,
        approver_id: UserId,
        comment: &str,
        timestamp: DateTime<Utc>,
    ) -> Result<u64, AppError> {
        use crate::models::request::RequestStatus;

        let query = format!(
            "UPDATE {} SET status = $1, rejected_by = $2, rejected_at = $3, decision_comment = $4, \
             updated_at = $5 WHERE id = $6 AND status = 'pending'",
            "leave_requests"
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
    fn test_mock_leave_request_repository_can_be_created() {
        let _mock = MockLeaveRequestRepositoryTrait::new();
    }

    #[test]
    fn test_mock_leave_request_repository_trait_bounds() {
        fn check_send_sync<T: Send + Sync>() {}
        check_send_sync::<MockLeaveRequestRepositoryTrait>();
    }

    #[test]
    fn leave_request_repository_new_creates_instance() {
        let repo = LeaveRequestRepository::new();
        let _repo = repo;
    }
}
