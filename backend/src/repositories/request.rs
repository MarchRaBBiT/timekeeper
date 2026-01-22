use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, QueryBuilder};
use std::str::FromStr;

use crate::error::AppError;
use crate::models::{leave_request::LeaveRequest, overtime_request::OvertimeRequest};
use crate::repositories::{
    common::push_clause, repository::Repository, LeaveRequestRepository, OvertimeRequestRepository,
};
use crate::types::{LeaveRequestId, OvertimeRequestId, UserId};

/// Filters for querying request lists.
///
/// Used to filter leave and overtime requests by status, user, and date range.
#[derive(Debug, Clone, Default)]
pub struct RequestListFilters {
    /// Filter by request status (e.g., "pending", "approved", "rejected")
    pub status: Option<String>,
    /// Filter by user ID
    pub user_id: Option<String>,
    /// Filter requests created from this timestamp (inclusive)
    pub from: Option<DateTime<Utc>>,
    /// Filter requests created until this timestamp (inclusive)
    pub to: Option<DateTime<Utc>>,
}

/// Result container for leave and overtime request lists.
#[derive(Debug, Clone, Default)]
pub struct RequestListResult {
    /// List of leave requests matching the query
    pub leave_requests: Vec<LeaveRequest>,
    /// List of overtime requests matching the query
    pub overtime_requests: Vec<OvertimeRequest>,
}

/// Input type for creating a new request (leave or overtime).
pub enum RequestCreate<'a> {
    /// Create a leave request
    Leave(&'a LeaveRequest),
    /// Create an overtime request
    Overtime(&'a OvertimeRequest),
}

/// Output type representing a created request record.
pub enum RequestRecord {
    /// A created leave request
    Leave(LeaveRequest),
    /// A created overtime request
    Overtime(OvertimeRequest),
}

/// Update operation for changing request status (approve or reject).
pub enum RequestStatusUpdate<'a> {
    /// Approve a request with approver details and comment
    Approve {
        /// ID of the user approving the request
        approver_id: UserId,
        /// Decision comment explaining the approval
        comment: &'a str,
        /// Timestamp when the approval occurred
        timestamp: DateTime<Utc>,
    },
    /// Reject a request with approver details and comment
    Reject {
        /// ID of the user rejecting the request
        approver_id: UserId,
        /// Decision comment explaining the rejection
        comment: &'a str,
        /// Timestamp when the rejection occurred
        timestamp: DateTime<Utc>,
    },
}

/// Unified repository for managing leave and overtime requests.
///
/// Provides higher-level operations that delegate to specialized repositories
/// while offering polymorphic handling of both request types.
#[derive(Debug, Default, Clone, Copy)]
pub struct RequestRepository;

impl RequestRepository {
    /// Creates a new instance of the request repository.
    pub fn new() -> Self {
        Self
    }

    /// Retrieves paginated lists of leave and/or overtime requests with optional filtering.
    ///
    /// # Arguments
    ///
    /// * `db` - Database connection pool
    /// * `filters` - Filter criteria (status, user_id, date range)
    /// * `per_page` - Maximum number of results per request type
    /// * `offset` - Number of records to skip per request type
    /// * `include_leave` - Whether to query leave requests
    /// * `include_overtime` - Whether to query overtime requests
    ///
    /// # Returns
    ///
    /// A `RequestListResult` containing separate vectors for leave and overtime requests.
    /// Empty vectors are returned for excluded request types.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let filters = RequestListFilters {
    ///     status: Some("pending".to_string()),
    ///     ..Default::default()
    /// };
    /// let result = repo.get_requests_with_relations(&pool, &filters, 20, 0, true, true).await?;
    /// ```
    pub async fn get_requests_with_relations(
        &self,
        db: &PgPool,
        filters: &RequestListFilters,
        per_page: i64,
        offset: i64,
        include_leave: bool,
        include_overtime: bool,
    ) -> Result<RequestListResult, AppError> {
        let leave_requests = if include_leave {
            list_leave_requests(db, filters, per_page, offset).await?
        } else {
            Vec::new()
        };

        let overtime_requests = if include_overtime {
            list_overtime_requests(db, filters, per_page, offset).await?
        } else {
            Vec::new()
        };

        Ok(RequestListResult {
            leave_requests,
            overtime_requests,
        })
    }

    /// Retrieves all leave and overtime requests for a specific user.
    ///
    /// # Arguments
    ///
    /// * `db` - Database connection pool
    /// * `user_id` - ID of the user whose requests to retrieve
    ///
    /// # Returns
    ///
    /// A `RequestListResult` containing all leave and overtime requests for the user,
    /// ordered by creation date (descending).
    pub async fn get_user_requests(
        &self,
        db: &PgPool,
        user_id: UserId,
    ) -> Result<RequestListResult, AppError> {
        let leave_repo = LeaveRequestRepository::new();
        let overtime_repo = OvertimeRequestRepository::new();

        let leave_requests = leave_repo.find_by_user(db, user_id).await?;
        let overtime_requests = overtime_repo.find_by_user(db, user_id).await?;

        Ok(RequestListResult {
            leave_requests,
            overtime_requests,
        })
    }

    /// Creates a new leave or overtime request with automatic history tracking.
    ///
    /// # Arguments
    ///
    /// * `db` - Database connection pool
    /// * `request` - The request to create (either leave or overtime)
    ///
    /// # Returns
    ///
    /// The created request record wrapped in the corresponding variant.
    ///
    /// # Errors
    ///
    /// Returns `AppError` if the database operation fails.
    pub async fn create_request_with_history(
        &self,
        db: &PgPool,
        request: RequestCreate<'_>,
    ) -> Result<RequestRecord, AppError> {
        match request {
            RequestCreate::Leave(item) => {
                let repo = LeaveRequestRepository::new();
                let saved = repo.create(db, item).await?;
                Ok(RequestRecord::Leave(saved))
            }
            RequestCreate::Overtime(item) => {
                let repo = OvertimeRequestRepository::new();
                let saved = repo.create(db, item).await?;
                Ok(RequestRecord::Overtime(saved))
            }
        }
    }

    /// Updates the status of a request (approve or reject).
    ///
    /// Attempts to parse the request ID as either a leave or overtime request ID
    /// and applies the status update to the matching request type.
    ///
    /// # Arguments
    ///
    /// * `db` - Database connection pool
    /// * `request_id` - String representation of the request ID (leave or overtime)
    /// * `update` - The status update to apply (approve or reject with metadata)
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - If the request was found and successfully updated
    /// * `Ok(false)` - If the request ID was invalid or request not found
    ///
    /// # Errors
    ///
    /// Returns `AppError` if the database operation fails.
    pub async fn update_request_status(
        &self,
        db: &PgPool,
        request_id: &str,
        update: RequestStatusUpdate<'_>,
    ) -> Result<bool, AppError> {
        let leave_repo = LeaveRequestRepository::new();
        if let Ok(leave_request_id) = LeaveRequestId::from_str(request_id) {
            let affected = match &update {
                RequestStatusUpdate::Approve {
                    approver_id,
                    comment,
                    timestamp,
                } => {
                    leave_repo
                        .approve(
                            db,
                            leave_request_id,
                            *approver_id,
                            comment,
                            *timestamp,
                        )
                        .await?
                }
                RequestStatusUpdate::Reject {
                    approver_id,
                    comment,
                    timestamp,
                } => {
                    leave_repo
                        .reject(
                            db,
                            leave_request_id,
                            *approver_id,
                            comment,
                            *timestamp,
                        )
                        .await?
                }
            };

            if affected > 0 {
                return Ok(true);
            }
        }

        let overtime_repo = OvertimeRequestRepository::new();
        if let Ok(overtime_request_id) = OvertimeRequestId::from_str(request_id) {
            let affected = match &update {
                RequestStatusUpdate::Approve {
                    approver_id,
                    comment,
                    timestamp,
                } => {
                    overtime_repo
                        .approve(
                            db,
                            overtime_request_id,
                            *approver_id,
                            comment,
                            *timestamp,
                        )
                        .await?
                }
                RequestStatusUpdate::Reject {
                    approver_id,
                    comment,
                    timestamp,
                } => {
                    overtime_repo
                        .reject(
                            db,
                            overtime_request_id,
                            *approver_id,
                            comment,
                            *timestamp,
                        )
                        .await?
                }
            };

            if affected > 0 {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

async fn list_leave_requests(
    db: &PgPool,
    filters: &RequestListFilters,
    per_page: i64,
    offset: i64,
) -> Result<Vec<LeaveRequest>, AppError> {
    let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT id, user_id, leave_type, start_date, end_date, reason, status, approved_by, approved_at, rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at FROM leave_requests",
    );
    apply_request_filters(&mut builder, filters);
    builder
        .push(" ORDER BY created_at DESC LIMIT ")
        .push_bind(per_page)
        .push(" OFFSET ")
        .push_bind(offset);
    builder
        .build_query_as::<LeaveRequest>()
        .fetch_all(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))
}

async fn list_overtime_requests(
    db: &PgPool,
    filters: &RequestListFilters,
    per_page: i64,
    offset: i64,
) -> Result<Vec<OvertimeRequest>, AppError> {
    let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT id, user_id, date, planned_hours, reason, status, approved_by, approved_at, rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at FROM overtime_requests",
    );
    apply_request_filters(&mut builder, filters);
    builder
        .push(" ORDER BY created_at DESC LIMIT ")
        .push_bind(per_page)
        .push(" OFFSET ")
        .push_bind(offset);
    builder
        .build_query_as::<OvertimeRequest>()
        .fetch_all(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))
}

fn apply_request_filters<'a>(
    builder: &mut QueryBuilder<'a, Postgres>,
    filters: &'a RequestListFilters,
) {
    let mut has_clause = false;
    if let Some(ref uid) = filters.user_id {
        push_clause(builder, &mut has_clause);
        builder.push("user_id = ").push_bind(uid);
    }
    if let Some(ref status) = filters.status {
        push_clause(builder, &mut has_clause);
        builder.push("status = ").push_bind(status);
    }
    if let Some(from) = filters.from.as_ref() {
        push_clause(builder, &mut has_clause);
        builder.push("created_at >= ").push_bind(*from);
    }
    if let Some(to) = filters.to.as_ref() {
        push_clause(builder, &mut has_clause);
        builder.push("created_at <= ").push_bind(*to);
    }
}
