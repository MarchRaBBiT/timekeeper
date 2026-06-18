use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, PgPool};
use timekeeper_app::attendance::{
    AdminAttendanceCorrectionListFilters, AdminAttendanceCorrectionReadError,
    ApprovedAttendanceCorrectionRequest, AttendanceCorrectionBreak,
    AttendanceCorrectionDecisionError, AttendanceCorrectionRecord,
    AttendanceCorrectionRequestStatus, AttendanceCorrectionSnapshot,
    CancelAttendanceCorrectionRepository, CorrectionAttendance, CreateAttendanceCorrectionError,
    CreateAttendanceCorrectionRepository, DecideAttendanceCorrectionRepository,
    ListAdminAttendanceCorrectionRequestsRepository, NewAttendanceCorrectionRequest,
    RejectedAttendanceCorrectionRequest, UpdateAttendanceCorrectionRepository,
    UpdatedAttendanceCorrectionRequest,
};
use uuid::Uuid;

const REQUEST_COLUMNS: &str = "id, user_id, attendance_id, date, status, reason,
    original_snapshot_json, proposed_values_json, decision_comment,
    approved_by, approved_at, rejected_by, rejected_at, cancelled_at,
    created_at, updated_at";

#[derive(Debug, Clone)]
pub struct AttendanceCorrectionRepository {
    pool: PgPool,
}

impl AttendanceCorrectionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn list_by_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<AttendanceCorrectionRecord>, CreateAttendanceCorrectionError> {
        validate_uuid(user_id)
            .map_err(|_| CreateAttendanceCorrectionError::Repository("invalid user_id".into()))?;
        let rows = sqlx::query_as::<_, AttendanceCorrectionRequestRow>(&format!(
            "SELECT {REQUEST_COLUMNS}
             FROM attendance_correction_requests
             WHERE user_id = $1
             ORDER BY created_at DESC"
        ))
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(create_repository_error)?;

        rows.into_iter().map(row_to_record_for_create).collect()
    }
}

#[derive(Debug, Clone, FromRow)]
struct AttendanceCorrectionRequestRow {
    id: String,
    user_id: String,
    attendance_id: String,
    date: NaiveDate,
    status: String,
    reason: String,
    original_snapshot_json: Value,
    proposed_values_json: Value,
    decision_comment: Option<String>,
    approved_by: Option<String>,
    approved_at: Option<DateTime<Utc>>,
    rejected_by: Option<String>,
    rejected_at: Option<DateTime<Utc>>,
    cancelled_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
struct CorrectionAttendanceRow {
    id: String,
    user_id: String,
    date: NaiveDate,
    clock_in_time: Option<NaiveDateTime>,
    clock_out_time: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, FromRow)]
struct CorrectionBreakRow {
    break_start_time: NaiveDateTime,
    break_end_time: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CorrectionSnapshotJson {
    clock_in_time: Option<NaiveDateTime>,
    clock_out_time: Option<NaiveDateTime>,
    breaks: Vec<CorrectionBreakJson>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CorrectionBreakJson {
    break_start_time: NaiveDateTime,
    break_end_time: Option<NaiveDateTime>,
}

#[async_trait]
impl CreateAttendanceCorrectionRepository for AttendanceCorrectionRepository {
    async fn find_attendance_by_user_and_date(
        &self,
        user_id: &str,
        date: NaiveDate,
    ) -> Result<Option<CorrectionAttendance>, CreateAttendanceCorrectionError> {
        validate_uuid(user_id)
            .map_err(|_| CreateAttendanceCorrectionError::Repository("invalid user_id".into()))?;
        sqlx::query_as::<_, CorrectionAttendanceRow>(
            "SELECT id, user_id, date, clock_in_time, clock_out_time
             FROM attendance
             WHERE user_id = $1 AND date = $2",
        )
        .bind(user_id)
        .bind(date)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(correction_attendance_to_app))
        .map_err(create_repository_error)
    }

    async fn breaks_for_attendance(
        &self,
        attendance_id: &str,
    ) -> Result<Vec<AttendanceCorrectionBreak>, CreateAttendanceCorrectionError> {
        validate_uuid(attendance_id).map_err(|_| {
            CreateAttendanceCorrectionError::Repository("invalid attendance_id".into())
        })?;
        sqlx::query_as::<_, CorrectionBreakRow>(
            "SELECT break_start_time, break_end_time
             FROM break_records
             WHERE attendance_id = $1
             ORDER BY break_start_time ASC",
        )
        .bind(attendance_id)
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(correction_break_to_app).collect())
        .map_err(create_repository_error)
    }

    async fn create_attendance_correction_request(
        &self,
        request: NewAttendanceCorrectionRequest,
    ) -> Result<AttendanceCorrectionRecord, CreateAttendanceCorrectionError> {
        validate_uuid(&request.id).map_err(|_| {
            CreateAttendanceCorrectionError::Repository("invalid request_id".into())
        })?;
        validate_uuid(&request.user_id)
            .map_err(|_| CreateAttendanceCorrectionError::Repository("invalid user_id".into()))?;
        validate_uuid(&request.attendance_id).map_err(|_| {
            CreateAttendanceCorrectionError::Repository("invalid attendance_id".into())
        })?;

        let now = Utc::now();
        let original_snapshot_json =
            app_snapshot_to_json_value(request.original_snapshot).map_err(create_snapshot_error)?;
        let proposed_values_json =
            app_snapshot_to_json_value(request.proposed_values).map_err(create_snapshot_error)?;

        let row = sqlx::query_as::<_, AttendanceCorrectionRequestRow>(&format!(
            "INSERT INTO attendance_correction_requests (
                id, user_id, attendance_id, date, status, reason,
                original_snapshot_json, proposed_values_json, created_at, updated_at
            ) VALUES ($1,$2,$3,$4,'pending',$5,$6,$7,$8,$9)
            RETURNING {REQUEST_COLUMNS}"
        ))
        .bind(request.id)
        .bind(request.user_id)
        .bind(request.attendance_id)
        .bind(request.date)
        .bind(request.reason)
        .bind(original_snapshot_json)
        .bind(proposed_values_json)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(create_repository_error)?;

        row_to_record_for_create(row)
    }
}

#[async_trait]
impl UpdateAttendanceCorrectionRepository for AttendanceCorrectionRepository {
    async fn find_attendance_correction_request_for_user(
        &self,
        request_id: &str,
        user_id: &str,
    ) -> Result<AttendanceCorrectionRecord, CreateAttendanceCorrectionError> {
        validate_uuid(user_id)
            .map_err(|_| CreateAttendanceCorrectionError::Repository("invalid user_id".into()))?;
        let row = sqlx::query_as::<_, AttendanceCorrectionRequestRow>(&format!(
            "SELECT {REQUEST_COLUMNS}
             FROM attendance_correction_requests
             WHERE id = $1 AND user_id = $2"
        ))
        .bind(request_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(create_repository_error)?
        .ok_or(CreateAttendanceCorrectionError::RequestNotFound)?;

        row_to_record_for_create(row)
    }

    async fn update_pending_attendance_correction_request(
        &self,
        request: UpdatedAttendanceCorrectionRequest,
    ) -> Result<AttendanceCorrectionRecord, CreateAttendanceCorrectionError> {
        validate_uuid(&request.user_id)
            .map_err(|_| CreateAttendanceCorrectionError::Repository("invalid user_id".into()))?;
        let now = Utc::now();
        let proposed_values_json =
            app_snapshot_to_json_value(request.proposed_values).map_err(create_snapshot_error)?;

        let row = sqlx::query_as::<_, AttendanceCorrectionRequestRow>(&format!(
            "UPDATE attendance_correction_requests
             SET reason = $1, proposed_values_json = $2, updated_at = $3
             WHERE id = $4 AND user_id = $5 AND status = 'pending'
             RETURNING {REQUEST_COLUMNS}"
        ))
        .bind(request.reason)
        .bind(proposed_values_json)
        .bind(now)
        .bind(request.id)
        .bind(request.user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(create_repository_error)?
        .ok_or(CreateAttendanceCorrectionError::NotPendingUpdate)?;

        row_to_record_for_create(row)
    }
}

#[async_trait]
impl CancelAttendanceCorrectionRepository for AttendanceCorrectionRepository {
    async fn cancel_pending_attendance_correction_request(
        &self,
        request_id: &str,
        user_id: &str,
    ) -> Result<(), CreateAttendanceCorrectionError> {
        validate_uuid(user_id)
            .map_err(|_| CreateAttendanceCorrectionError::Repository("invalid user_id".into()))?;
        let now = Utc::now();
        let affected = sqlx::query(
            "UPDATE attendance_correction_requests
             SET status = 'cancelled', cancelled_at = $1, updated_at = $1
             WHERE id = $2 AND user_id = $3 AND status = 'pending'",
        )
        .bind(now)
        .bind(request_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(create_repository_error)?
        .rows_affected();

        if affected == 0 {
            return Err(CreateAttendanceCorrectionError::NotPendingCancel);
        }
        Ok(())
    }
}

#[async_trait]
impl DecideAttendanceCorrectionRepository for AttendanceCorrectionRepository {
    async fn find_attendance_correction_request(
        &self,
        request_id: &str,
    ) -> Result<AttendanceCorrectionRecord, AttendanceCorrectionDecisionError> {
        let row = find_request_by_id(&self.pool, request_id)
            .await
            .map_err(decision_find_error)?;
        row_to_record_for_decision(row)
    }

    async fn can_manager_approve(
        &self,
        manager_id: &str,
        applicant_id: &str,
    ) -> Result<bool, AttendanceCorrectionDecisionError> {
        validate_uuid(manager_id).map_err(|_| {
            AttendanceCorrectionDecisionError::Repository("invalid manager_id".into())
        })?;
        validate_uuid(applicant_id).map_err(|_| {
            AttendanceCorrectionDecisionError::Repository("invalid applicant_id".into())
        })?;
        can_manager_approve(&self.pool, manager_id, applicant_id)
            .await
            .map_err(decision_repository_error)
    }

    async fn approve_attendance_correction_request(
        &self,
        request: ApprovedAttendanceCorrectionRequest,
    ) -> Result<(), AttendanceCorrectionDecisionError> {
        validate_uuid(&request.attendance_id).map_err(|_| {
            AttendanceCorrectionDecisionError::Repository("invalid attendance_id".into())
        })?;
        validate_uuid(&request.approver_id).map_err(|_| {
            AttendanceCorrectionDecisionError::Repository("invalid approver_id".into())
        })?;

        approve_and_apply_effective_values(&self.pool, request).await
    }

    async fn reject_attendance_correction_request(
        &self,
        request: RejectedAttendanceCorrectionRequest,
    ) -> Result<(), AttendanceCorrectionDecisionError> {
        validate_uuid(&request.approver_id).map_err(|_| {
            AttendanceCorrectionDecisionError::Repository("invalid approver_id".into())
        })?;
        let now = Utc::now();
        let affected = sqlx::query(
            "UPDATE attendance_correction_requests
             SET status = 'rejected', rejected_by = $1, rejected_at = $2,
                 decision_comment = $3, updated_at = $2
             WHERE id = $4 AND status = 'pending'",
        )
        .bind(request.approver_id)
        .bind(now)
        .bind(request.comment)
        .bind(request.id)
        .execute(&self.pool)
        .await
        .map_err(decision_repository_error)?
        .rows_affected();

        if affected == 0 {
            return Err(AttendanceCorrectionDecisionError::AlreadyProcessed);
        }
        Ok(())
    }
}

#[async_trait]
impl ListAdminAttendanceCorrectionRequestsRepository for AttendanceCorrectionRepository {
    async fn list_subordinate_user_ids(
        &self,
        manager_id: &str,
    ) -> Result<Vec<String>, AdminAttendanceCorrectionReadError> {
        validate_uuid(manager_id).map_err(|_| {
            AdminAttendanceCorrectionReadError::Repository("invalid manager_id".into())
        })?;
        list_subordinate_user_ids(&self.pool, manager_id)
            .await
            .map_err(read_repository_error)
    }

    async fn list_admin_attendance_correction_requests(
        &self,
        filters: AdminAttendanceCorrectionListFilters,
    ) -> Result<Vec<AttendanceCorrectionRecord>, AdminAttendanceCorrectionReadError> {
        if let Some(user_id) = filters.user_id.as_deref() {
            validate_uuid(user_id)
                .map_err(|_| AdminAttendanceCorrectionReadError::InvalidUserId)?;
        }
        let offset = (filters.page - 1).max(0) * filters.per_page;
        let rows = sqlx::query_as::<_, AttendanceCorrectionRequestRow>(&format!(
            "SELECT {REQUEST_COLUMNS}
             FROM attendance_correction_requests
             WHERE ($1::text IS NULL OR status = $1)
               AND ($2::uuid IS NULL OR user_id = $2)
               AND ($3::text[] IS NULL OR user_id::text = ANY($3))
             ORDER BY created_at DESC
             LIMIT $4 OFFSET $5"
        ))
        .bind(filters.status)
        .bind(filters.user_id)
        .bind(filters.allowed_user_ids)
        .bind(filters.per_page)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(read_repository_error)?;

        rows.into_iter().map(row_to_record_for_read).collect()
    }

    async fn find_admin_attendance_correction_request(
        &self,
        request_id: &str,
    ) -> Result<AttendanceCorrectionRecord, AdminAttendanceCorrectionReadError> {
        let row = find_request_by_id(&self.pool, request_id)
            .await
            .map_err(read_find_error)?;
        row_to_record_for_read(row)
    }

    async fn can_manager_view_request(
        &self,
        manager_id: &str,
        applicant_id: &str,
    ) -> Result<bool, AdminAttendanceCorrectionReadError> {
        validate_uuid(manager_id).map_err(|_| {
            AdminAttendanceCorrectionReadError::Repository("invalid manager_id".into())
        })?;
        validate_uuid(applicant_id).map_err(|_| {
            AdminAttendanceCorrectionReadError::Repository("invalid applicant_id".into())
        })?;
        can_manager_approve(&self.pool, manager_id, applicant_id)
            .await
            .map_err(read_repository_error)
    }
}

async fn find_request_by_id(
    pool: &PgPool,
    request_id: &str,
) -> Result<AttendanceCorrectionRequestRow, sqlx::Error> {
    sqlx::query_as::<_, AttendanceCorrectionRequestRow>(&format!(
        "SELECT {REQUEST_COLUMNS}
         FROM attendance_correction_requests
         WHERE id = $1"
    ))
    .bind(request_id)
    .fetch_optional(pool)
    .await?
    .ok_or(sqlx::Error::RowNotFound)
}

async fn approve_and_apply_effective_values(
    pool: &PgPool,
    request: ApprovedAttendanceCorrectionRequest,
) -> Result<(), AttendanceCorrectionDecisionError> {
    let now = Utc::now();
    let breaks_json = serde_json::to_value(
        CorrectionSnapshotJson::from_app(AttendanceCorrectionSnapshot {
            clock_in_time: request.proposed_values.clock_in_time,
            clock_out_time: request.proposed_values.clock_out_time,
            breaks: request.proposed_values.breaks.clone(),
        })
        .breaks,
    )
    .map_err(|error| AttendanceCorrectionDecisionError::Repository(error.to_string()))?;

    let mut tx = pool.begin().await.map_err(decision_repository_error)?;
    let latest_attendance = sqlx::query_as::<_, (Option<NaiveDateTime>, Option<NaiveDateTime>)>(
        "SELECT clock_in_time, clock_out_time
             FROM attendance
             WHERE id = $1
             FOR UPDATE",
    )
    .bind(&request.attendance_id)
    .fetch_optional(tx.as_mut())
    .await
    .map_err(decision_repository_error)?;

    let Some((clock_in_time, clock_out_time)) = latest_attendance else {
        return Err(AttendanceCorrectionDecisionError::RequestNotFound);
    };

    let latest_breaks = sqlx::query_as::<_, CorrectionBreakRow>(
        "SELECT break_start_time, break_end_time
         FROM break_records
         WHERE attendance_id = $1
         ORDER BY break_start_time ASC
         FOR UPDATE",
    )
    .bind(&request.attendance_id)
    .fetch_all(tx.as_mut())
    .await
    .map_err(decision_repository_error)?;

    let latest_snapshot = AttendanceCorrectionSnapshot {
        clock_in_time,
        clock_out_time,
        breaks: latest_breaks
            .into_iter()
            .map(correction_break_to_app)
            .collect(),
    };
    if latest_snapshot != request.original_snapshot {
        return Err(AttendanceCorrectionDecisionError::AttendanceChanged);
    }

    let affected = sqlx::query(
        "UPDATE attendance_correction_requests
         SET status = 'approved', approved_by = $1, approved_at = $2,
             decision_comment = $3, updated_at = $2
         WHERE id = $4 AND status = 'pending'",
    )
    .bind(&request.approver_id)
    .bind(now)
    .bind(&request.comment)
    .bind(&request.id)
    .execute(tx.as_mut())
    .await
    .map_err(decision_repository_error)?
    .rows_affected();

    if affected == 0 {
        return Err(AttendanceCorrectionDecisionError::AlreadyProcessed);
    }

    sqlx::query(
        "INSERT INTO attendance_correction_effective_values (
            attendance_id, source_request_id,
            clock_in_time_corrected, clock_out_time_corrected,
            break_records_corrected_json, applied_by, applied_at, updated_at
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$7)
        ON CONFLICT(attendance_id) DO UPDATE SET
            source_request_id = EXCLUDED.source_request_id,
            clock_in_time_corrected = EXCLUDED.clock_in_time_corrected,
            clock_out_time_corrected = EXCLUDED.clock_out_time_corrected,
            break_records_corrected_json = EXCLUDED.break_records_corrected_json,
            applied_by = EXCLUDED.applied_by,
            applied_at = EXCLUDED.applied_at,
            updated_at = EXCLUDED.updated_at",
    )
    .bind(&request.attendance_id)
    .bind(&request.id)
    .bind(request.proposed_values.clock_in_time)
    .bind(request.proposed_values.clock_out_time)
    .bind(breaks_json)
    .bind(&request.approver_id)
    .bind(now)
    .execute(tx.as_mut())
    .await
    .map_err(decision_repository_error)?;

    tx.commit().await.map_err(decision_repository_error)
}

async fn can_manager_approve(
    pool: &PgPool,
    manager_id: &str,
    applicant_id: &str,
) -> Result<bool, sqlx::Error> {
    let result: (bool,) = sqlx::query_as(
        r#"
        WITH RECURSIVE subordinate_depts AS (
            SELECT dm.department_id
            FROM department_managers dm
            WHERE dm.user_id = $1
            UNION ALL
            SELECT d.id
            FROM departments d
            INNER JOIN subordinate_depts sd ON d.parent_id = sd.department_id
        )
        SELECT EXISTS (
            SELECT 1 FROM users u
            WHERE u.id = $2
              AND u.department_id IN (SELECT department_id FROM subordinate_depts)
        ) AS can_approve
        "#,
    )
    .bind(manager_id)
    .bind(applicant_id)
    .fetch_one(pool)
    .await?;
    Ok(result.0)
}

async fn list_subordinate_user_ids(
    pool: &PgPool,
    manager_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    let rows: Vec<(String,)> = sqlx::query_as(
        r#"
        WITH RECURSIVE subordinate_depts AS (
            SELECT dm.department_id
            FROM department_managers dm
            WHERE dm.user_id = $1
            UNION ALL
            SELECT d.id
            FROM departments d
            INNER JOIN subordinate_depts sd ON d.parent_id = sd.department_id
        )
        SELECT u.id
        FROM users u
        WHERE u.department_id IN (SELECT department_id FROM subordinate_depts)
        "#,
    )
    .bind(manager_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

fn correction_attendance_to_app(row: CorrectionAttendanceRow) -> CorrectionAttendance {
    CorrectionAttendance {
        attendance_id: row.id,
        user_id: row.user_id,
        date: row.date,
        clock_in_time: row.clock_in_time,
        clock_out_time: row.clock_out_time,
    }
}

fn correction_break_to_app(row: CorrectionBreakRow) -> AttendanceCorrectionBreak {
    AttendanceCorrectionBreak {
        break_start_time: row.break_start_time,
        break_end_time: row.break_end_time,
    }
}

fn row_to_record(
    row: AttendanceCorrectionRequestRow,
) -> Result<AttendanceCorrectionRecord, String> {
    Ok(AttendanceCorrectionRecord {
        id: row.id,
        user_id: row.user_id,
        attendance_id: row.attendance_id,
        date: row.date,
        status: status_to_app(&row.status)?,
        reason: row.reason,
        original_snapshot: json_value_to_app_snapshot(row.original_snapshot_json)?,
        proposed_values: json_value_to_app_snapshot(row.proposed_values_json)?,
        decision_comment: row.decision_comment,
        approved_by: row.approved_by,
        approved_at: row.approved_at,
        rejected_by: row.rejected_by,
        rejected_at: row.rejected_at,
        cancelled_at: row.cancelled_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

fn row_to_record_for_create(
    row: AttendanceCorrectionRequestRow,
) -> Result<AttendanceCorrectionRecord, CreateAttendanceCorrectionError> {
    row_to_record(row).map_err(CreateAttendanceCorrectionError::Repository)
}

fn row_to_record_for_decision(
    row: AttendanceCorrectionRequestRow,
) -> Result<AttendanceCorrectionRecord, AttendanceCorrectionDecisionError> {
    row_to_record(row).map_err(AttendanceCorrectionDecisionError::Repository)
}

fn row_to_record_for_read(
    row: AttendanceCorrectionRequestRow,
) -> Result<AttendanceCorrectionRecord, AdminAttendanceCorrectionReadError> {
    row_to_record(row).map_err(AdminAttendanceCorrectionReadError::Repository)
}

fn status_to_app(status: &str) -> Result<AttendanceCorrectionRequestStatus, String> {
    match status {
        "pending" => Ok(AttendanceCorrectionRequestStatus::Pending),
        "approved" => Ok(AttendanceCorrectionRequestStatus::Approved),
        "rejected" => Ok(AttendanceCorrectionRequestStatus::Rejected),
        "cancelled" => Ok(AttendanceCorrectionRequestStatus::Cancelled),
        "conflict" => Ok(AttendanceCorrectionRequestStatus::Conflict),
        other => Err(format!("unknown attendance correction status: {other}")),
    }
}

fn app_snapshot_to_json_value(
    snapshot: AttendanceCorrectionSnapshot,
) -> Result<Value, serde_json::Error> {
    serde_json::to_value(CorrectionSnapshotJson::from_app(snapshot))
}

fn json_value_to_app_snapshot(value: Value) -> Result<AttendanceCorrectionSnapshot, String> {
    serde_json::from_value::<CorrectionSnapshotJson>(value)
        .map(CorrectionSnapshotJson::into_app)
        .map_err(|error| error.to_string())
}

impl CorrectionSnapshotJson {
    fn from_app(snapshot: AttendanceCorrectionSnapshot) -> Self {
        Self {
            clock_in_time: snapshot.clock_in_time,
            clock_out_time: snapshot.clock_out_time,
            breaks: snapshot
                .breaks
                .into_iter()
                .map(|item| CorrectionBreakJson {
                    break_start_time: item.break_start_time,
                    break_end_time: item.break_end_time,
                })
                .collect(),
        }
    }

    fn into_app(self) -> AttendanceCorrectionSnapshot {
        AttendanceCorrectionSnapshot {
            clock_in_time: self.clock_in_time,
            clock_out_time: self.clock_out_time,
            breaks: self
                .breaks
                .into_iter()
                .map(|item| AttendanceCorrectionBreak {
                    break_start_time: item.break_start_time,
                    break_end_time: item.break_end_time,
                })
                .collect(),
        }
    }
}

fn validate_uuid(value: &str) -> Result<(), uuid::Error> {
    Uuid::parse_str(value).map(|_| ())
}

fn create_snapshot_error(error: serde_json::Error) -> CreateAttendanceCorrectionError {
    CreateAttendanceCorrectionError::Repository(error.to_string())
}

fn create_repository_error(error: sqlx::Error) -> CreateAttendanceCorrectionError {
    CreateAttendanceCorrectionError::Repository(error.to_string())
}

fn decision_repository_error(error: sqlx::Error) -> AttendanceCorrectionDecisionError {
    AttendanceCorrectionDecisionError::Repository(error.to_string())
}

fn decision_find_error(error: sqlx::Error) -> AttendanceCorrectionDecisionError {
    match error {
        sqlx::Error::RowNotFound => AttendanceCorrectionDecisionError::RequestNotFound,
        other => decision_repository_error(other),
    }
}

fn read_repository_error(error: sqlx::Error) -> AdminAttendanceCorrectionReadError {
    AdminAttendanceCorrectionReadError::Repository(error.to_string())
}

fn read_find_error(error: sqlx::Error) -> AdminAttendanceCorrectionReadError {
    match error {
        sqlx::Error::RowNotFound => AdminAttendanceCorrectionReadError::RequestNotFound,
        other => read_repository_error(other),
    }
}
