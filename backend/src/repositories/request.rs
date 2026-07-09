use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, QueryBuilder};
use std::str::FromStr;
use timekeeper_app::leave_ledger::{
    build_annual_leave_consume_entries, build_annual_leave_release_entries,
    ensure_annual_leave_request_has_balance, AnnualLeaveRequestLedgerCommand, LeaveLedgerError,
    LeaveLedgerRepository, ANNUAL_LEAVE_TYPE,
};
use timekeeper_contract::leave::{
    LEAVE_BALANCE_INSUFFICIENT_CODE, LEAVE_REQUEST_NO_ACTIVE_LOT_CODE,
    LEAVE_REQUEST_NO_WORKING_DAYS_CODE,
};
use timekeeper_infra_postgres::leave_ledger::LeaveLedgerPostgresRepository;

use crate::error::AppError;
use crate::models::{
    leave_request::{LeaveRequest, LeaveType},
    overtime_request::OvertimeRequest,
    request::RequestStatus,
};
use crate::repositories::{
    annual_leave_workdays::count_working_days_for_annual_leave,
    common::push_clause,
    leave_request::{LeaveRequestRepository, LeaveRequestRepositoryTrait},
    overtime_request::{OvertimeRequestRepository, OvertimeRequestRepositoryTrait},
    repository::Repository,
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
    /// Restrict results to requests from these user IDs (used for manager department scope).
    /// When `None`, no restriction applies.
    pub allowed_user_ids: Option<Vec<String>>,
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

    pub async fn ensure_annual_leave_request_balance(
        &self,
        db: &PgPool,
        request: &LeaveRequest,
    ) -> Result<(), AppError> {
        if !matches!(request.leave_type, LeaveType::Annual) {
            return Ok(());
        }
        let workday_count = count_working_days_for_annual_leave(
            db,
            &request.user_id.to_string(),
            request.start_date,
            request.end_date,
        )
        .await?;
        let ledger = LeaveLedgerPostgresRepository::new(db.clone());
        let entries = ledger
            .list_entries(&request.user_id.to_string(), ANNUAL_LEAVE_TYPE)
            .await
            .map_err(leave_ledger_error_to_app_error)?;
        ensure_annual_leave_request_has_balance(
            &entries,
            request.start_date,
            request.end_date,
            workday_count,
        )
        .map_err(leave_ledger_error_to_app_error)
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
            if let Some(existing) = find_leave_request_optional(db, leave_request_id).await? {
                let affected = match &update {
                    RequestStatusUpdate::Approve {
                        approver_id,
                        comment,
                        timestamp,
                    } if matches!(existing.leave_type, LeaveType::Annual) => {
                        if self
                            .approve_annual_leave_request_with_ledger(
                                db,
                                leave_request_id,
                                *approver_id,
                                comment,
                                *timestamp,
                            )
                            .await?
                        {
                            1
                        } else {
                            0
                        }
                    }
                    RequestStatusUpdate::Approve {
                        approver_id,
                        comment,
                        timestamp,
                    } => {
                        leave_repo
                            .approve(db, leave_request_id, *approver_id, comment, *timestamp)
                            .await?
                    }
                    RequestStatusUpdate::Reject {
                        approver_id,
                        comment,
                        timestamp,
                    } => {
                        leave_repo
                            .reject(db, leave_request_id, *approver_id, comment, *timestamp)
                            .await?
                    }
                };

                return Ok(affected > 0);
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
                        .approve(db, overtime_request_id, *approver_id, comment, *timestamp)
                        .await?
                }
                RequestStatusUpdate::Reject {
                    approver_id,
                    comment,
                    timestamp,
                } => {
                    overtime_repo
                        .reject(db, overtime_request_id, *approver_id, comment, *timestamp)
                        .await?
                }
            };

            if affected > 0 {
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn approve_annual_leave_request_with_ledger(
        &self,
        db: &PgPool,
        id: LeaveRequestId,
        approver_id: UserId,
        comment: &str,
        timestamp: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        // 稼働日解決は resolved_workdays への materialize（書き込み）を伴うため、
        // ledger の行ロックを取る前に db 直下で行う。申請作成時
        // （ensure_annual_leave_request_balance）と同じ関数を使い、稼働日数の
        // 計算を乖離させない。
        let Some(pending_request) = find_leave_request_optional(db, id).await? else {
            return Ok(false);
        };
        if !matches!(pending_request.status, RequestStatus::Pending) {
            return Ok(false);
        }
        let workday_count = count_working_days_for_annual_leave(
            db,
            &pending_request.user_id.to_string(),
            pending_request.start_date,
            pending_request.end_date,
        )
        .await?;

        let mut tx = db.begin().await?;
        let Some(request) = find_leave_request_for_update(&mut tx, id).await? else {
            return Ok(false);
        };
        if !matches!(request.status, RequestStatus::Pending) {
            return Ok(false);
        }
        // workday_count はロック前に読んだ日付で解決している。ロックまでの間に
        // 申請が編集されて期間が変わっていた場合、消化分数と期間が食い違うため
        // conflict として拒否する（再実行すれば新しい期間で解決し直される）。
        if request.start_date != pending_request.start_date
            || request.end_date != pending_request.end_date
            || request.user_id != pending_request.user_id
        {
            return Err(AppError::Conflict(
                "leave request was modified while approving; please retry".to_string(),
            ));
        }
        lock_user_for_ledger(&mut tx, request.user_id).await?;
        let entries = LeaveLedgerPostgresRepository::list_entries_for_update(
            &mut tx,
            &request.user_id.to_string(),
            ANNUAL_LEAVE_TYPE,
        )
        .await
        .map_err(leave_ledger_error_to_app_error)?;
        let consume_entries = build_annual_leave_consume_entries(
            &entries,
            AnnualLeaveRequestLedgerCommand {
                user_id: request.user_id.to_string(),
                request_id: request.id.to_string(),
                start_date: request.start_date,
                end_date: request.end_date,
                created_by: Some(approver_id.to_string()),
            },
            workday_count,
        )
        .map_err(leave_ledger_error_to_app_error)?;

        let affected = sqlx::query(
            "UPDATE leave_requests
             SET status = $1, approved_by = $2, approved_at = $3, decision_comment = $4,
                 updated_at = $5
             WHERE id = $6 AND status = 'pending'",
        )
        .bind(RequestStatus::Approved.db_value())
        .bind(approver_id)
        .bind(timestamp)
        .bind(comment)
        .bind(timestamp)
        .bind(id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if affected == 0 {
            return Ok(false);
        }
        LeaveLedgerPostgresRepository::append_entries_in_transaction(&mut tx, consume_entries)
            .await
            .map_err(leave_ledger_error_to_app_error)?;
        tx.commit().await?;
        Ok(true)
    }

    pub async fn cancel_leave_request_with_ledger(
        &self,
        db: &PgPool,
        id: LeaveRequestId,
        user_id: UserId,
        timestamp: DateTime<Utc>,
    ) -> Result<u64, AppError> {
        let mut tx = db.begin().await?;
        let Some(request) = find_leave_request_for_update(&mut tx, id).await? else {
            return Ok(0);
        };
        if request.user_id != user_id {
            return Ok(0);
        }

        match request.status {
            RequestStatus::Pending => {
                let affected =
                    cancel_leave_request_in_tx(&mut tx, id, user_id, timestamp, "pending").await?;
                tx.commit().await?;
                Ok(affected)
            }
            RequestStatus::Approved if matches!(request.leave_type, LeaveType::Annual) => {
                lock_user_for_ledger(&mut tx, request.user_id).await?;
                let entries = LeaveLedgerPostgresRepository::list_entries_for_update(
                    &mut tx,
                    &request.user_id.to_string(),
                    ANNUAL_LEAVE_TYPE,
                )
                .await
                .map_err(leave_ledger_error_to_app_error)?;
                let release_entries = build_annual_leave_release_entries(
                    &entries,
                    AnnualLeaveRequestLedgerCommand {
                        user_id: request.user_id.to_string(),
                        request_id: request.id.to_string(),
                        start_date: request.start_date,
                        end_date: request.end_date,
                        created_by: Some(user_id.to_string()),
                    },
                );
                let affected =
                    cancel_leave_request_in_tx(&mut tx, id, user_id, timestamp, "approved").await?;
                if affected == 0 {
                    return Ok(0);
                }
                if !release_entries.is_empty() {
                    LeaveLedgerPostgresRepository::append_entries_in_transaction(
                        &mut tx,
                        release_entries,
                    )
                    .await
                    .map_err(leave_ledger_error_to_app_error)?;
                }
                tx.commit().await?;
                Ok(affected)
            }
            _ => Ok(0),
        }
    }
}

async fn find_leave_request_optional(
    db: &PgPool,
    id: LeaveRequestId,
) -> Result<Option<LeaveRequest>, AppError> {
    sqlx::query_as::<_, LeaveRequest>(
        "SELECT id, user_id, leave_type, start_date, end_date, reason, status,
                approved_by, approved_at, rejected_by, rejected_at, cancelled_at,
                decision_comment, created_at, updated_at
         FROM leave_requests
         WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(db)
    .await
    .map_err(AppError::from)
}

async fn find_leave_request_for_update(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    id: LeaveRequestId,
) -> Result<Option<LeaveRequest>, AppError> {
    sqlx::query_as::<_, LeaveRequest>(
        "SELECT id, user_id, leave_type, start_date, end_date, reason, status,
                approved_by, approved_at, rejected_by, rejected_at, cancelled_at,
                decision_comment, created_at, updated_at
         FROM leave_requests
         WHERE id = $1
         FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(AppError::from)
}

async fn lock_user_for_ledger(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    user_id: UserId,
) -> Result<(), AppError> {
    let found: Option<String> = sqlx::query_scalar("SELECT id FROM users WHERE id = $1 FOR UPDATE")
        .bind(user_id)
        .fetch_optional(&mut **tx)
        .await?;
    if found.is_none() {
        return Err(AppError::NotFound("User not found".into()));
    }
    Ok(())
}

async fn cancel_leave_request_in_tx(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    id: LeaveRequestId,
    user_id: UserId,
    timestamp: DateTime<Utc>,
    expected_status: &str,
) -> Result<u64, AppError> {
    sqlx::query(
        "UPDATE leave_requests
         SET status = $1, cancelled_at = $2, updated_at = $3
         WHERE id = $4 AND user_id = $5 AND status = $6",
    )
    .bind(RequestStatus::Cancelled.db_value())
    .bind(timestamp)
    .bind(timestamp)
    .bind(id)
    .bind(user_id)
    .bind(expected_status)
    .execute(&mut **tx)
    .await
    .map(|result| result.rows_affected())
    .map_err(AppError::from)
}

fn leave_ledger_error_to_app_error(error: LeaveLedgerError) -> AppError {
    match error {
        LeaveLedgerError::InvalidInput(message) => AppError::BadRequest(message),
        LeaveLedgerError::UserNotFound => AppError::NotFound("User not found".into()),
        LeaveLedgerError::RulesNotConfigured => {
            AppError::Conflict("Leave grant rules are not configured".into())
        }
        LeaveLedgerError::InsufficientBalance { .. } => AppError::BadRequestWithCode {
            message: "Insufficient annual leave balance".into(),
            code: LEAVE_BALANCE_INSUFFICIENT_CODE.to_string(),
        },
        LeaveLedgerError::NoWorkingDaysInRange => AppError::BadRequestWithCode {
            message: "Requested leave period has no working days to consume".into(),
            code: LEAVE_REQUEST_NO_WORKING_DAYS_CODE.to_string(),
        },
        LeaveLedgerError::NoActiveLeaveLot => AppError::BadRequestWithCode {
            message: "No active annual leave lot exists to determine day-equivalent minutes".into(),
            code: LEAVE_REQUEST_NO_ACTIVE_LOT_CODE.to_string(),
        },
        LeaveLedgerError::BatchAlreadyRunning => {
            AppError::Conflict("A leave grant batch is already running; please retry later".into())
        }
        LeaveLedgerError::Repository(message) => {
            AppError::InternalServerError(anyhow::anyhow!(message))
        }
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
    if let Some(ref uids) = filters.allowed_user_ids {
        push_clause(builder, &mut has_clause);
        builder
            .push("user_id = ANY(")
            .push_bind(uids.clone())
            .push(")");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use sqlx::{postgres::Postgres, QueryBuilder};

    #[test]
    fn apply_request_filters_without_filters_is_noop() {
        let mut builder = QueryBuilder::<Postgres>::new("SELECT 1");
        let filters = RequestListFilters::default();
        apply_request_filters(&mut builder, &filters);
    }

    #[test]
    fn apply_request_filters_with_all_fields_appends_clauses() {
        let mut builder = QueryBuilder::<Postgres>::new("SELECT 1");
        let now = Utc::now();
        let filters = RequestListFilters {
            user_id: Some("user-id".to_string()),
            status: Some("pending".to_string()),
            from: Some(now - Duration::days(1)),
            to: Some(now),
            allowed_user_ids: None,
        };
        apply_request_filters(&mut builder, &filters);
    }
}
