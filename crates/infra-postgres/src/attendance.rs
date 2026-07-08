use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde::Deserialize;
use serde_json::Value;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder, Row as _};
use timekeeper_app::attendance::{
    ActiveBreakSummary, AdminAttendanceExportFilters, AdminAttendanceExportRow, AdminLeaveDayRow,
    AttendanceDay, AttendanceRecord, AttendanceReplacement, AttendanceRepository,
    AttendanceStatusError, AttendanceStatusReadRepository, BreakEndError, BreakEndRepository,
    BreakPeriod, ClockInError, ClockOutError, ClockOutRepository, EffectiveAttendanceCorrection,
    EndedBreakPeriod, ExistingClockIn, ExistingClockOut, ExportAdminAttendanceError,
    ExportAdminAttendanceRepository, GetBreaksByAttendanceError, GetBreaksByAttendanceRepository,
    LeaveDayRecord, ListActiveBreaksError, ListActiveBreaksRepository, ListAttendancePageError,
    ListAttendancePageRepository, ListUserAttendanceError, ListUserAttendanceRepository,
    NewBreakPeriod, NewClockIn, StartBreakError, StartBreakRepository, UpsertAttendanceError,
    UpsertAttendanceRepository,
};
use timekeeper_domain::WorkDate;
use uuid::Uuid;

const ATTENDANCE_COLUMNS: &str = "id, user_id, date, clock_in_time, clock_out_time";
const BREAK_COLUMNS: &str = "id, attendance_id, break_start_time, break_end_time, duration_minutes";
const ACTIVE_BREAKS_LIST_LIMIT: i64 = 500;

#[derive(Debug, Clone)]
pub struct AttendanceWorkflowRepository {
    pool: PgPool,
}

impl AttendanceWorkflowRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

#[derive(Debug, Clone, FromRow)]
struct AttendanceRow {
    id: String,
    user_id: String,
    date: NaiveDate,
    clock_in_time: Option<NaiveDateTime>,
    clock_out_time: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, FromRow)]
struct AttendanceRecordRow {
    id: String,
    user_id: String,
    date: NaiveDate,
    clock_in_time: Option<NaiveDateTime>,
    clock_out_time: Option<NaiveDateTime>,
    status: String,
    total_work_hours: Option<f64>,
}

#[derive(Debug, Clone, FromRow)]
struct BreakRecordRow {
    id: String,
    attendance_id: String,
    break_start_time: NaiveDateTime,
    break_end_time: Option<NaiveDateTime>,
    duration_minutes: Option<i32>,
}

#[derive(Debug, Clone, FromRow)]
struct ActiveBreakRow {
    break_id: String,
    attendance_id: String,
    user_id: String,
    username: String,
    full_name: Option<String>,
    break_start_time: NaiveDateTime,
}

#[derive(Debug, Clone, FromRow)]
struct EffectiveCorrectionRow {
    attendance_id: String,
    clock_in_time_corrected: Option<NaiveDateTime>,
    clock_out_time_corrected: Option<NaiveDateTime>,
    break_records_corrected_json: Value,
}

#[derive(Debug, Clone, FromRow)]
struct AdminAttendanceExportRowData {
    username: String,
    full_name: String,
    date: NaiveDate,
    clock_in_time: Option<NaiveDateTime>,
    clock_out_time: Option<NaiveDateTime>,
    total_work_hours: Option<f64>,
    status: String,
}

#[derive(Debug, Clone, FromRow)]
struct LeaveDayRowData {
    leave_request_id: String,
    date: NaiveDate,
    leave_type: String,
}

#[derive(Debug, Clone, FromRow)]
struct AdminLeaveDayRowData {
    username: String,
    full_name: String,
    date: NaiveDate,
    leave_type: String,
}

#[derive(Debug, Clone, Deserialize)]
struct CorrectionBreakItemRow {
    break_start_time: NaiveDateTime,
    break_end_time: Option<NaiveDateTime>,
}

#[async_trait]
impl ExportAdminAttendanceRepository for AttendanceWorkflowRepository {
    async fn list_subordinate_user_ids(
        &self,
        manager_id: &str,
    ) -> Result<Vec<String>, ExportAdminAttendanceError> {
        validate_uuid(manager_id).map_err(export_admin_invalid_user_id)?;
        list_subordinate_user_ids(&self.pool, manager_id)
            .await
            .map_err(export_admin_repository_error)
    }

    async fn list_admin_attendance_export(
        &self,
        filters: AdminAttendanceExportFilters,
    ) -> Result<Vec<AdminAttendanceExportRow>, ExportAdminAttendanceError> {
        list_admin_attendance_export(&self.pool, filters)
            .await
            .map(|rows| rows.into_iter().map(admin_export_row_to_app).collect())
            .map_err(export_admin_repository_error)
    }

    async fn list_admin_leave_days(
        &self,
        filters: AdminAttendanceExportFilters,
    ) -> Result<Vec<AdminLeaveDayRow>, ExportAdminAttendanceError> {
        list_admin_leave_days(&self.pool, filters)
            .await
            .map(|rows| rows.into_iter().map(admin_leave_day_row_to_app).collect())
            .map_err(export_admin_repository_error)
    }
}

#[async_trait]
impl AttendanceRepository for AttendanceWorkflowRepository {
    async fn find_by_user_and_date(
        &self,
        user_id: &str,
        work_date: WorkDate,
    ) -> Result<Option<AttendanceDay>, ClockInError> {
        validate_uuid(user_id).map_err(clock_in_invalid_user_id)?;
        find_attendance_by_user_and_date(&self.pool, user_id, work_date.as_naive_date())
            .await
            .map(|attendance| attendance.map(attendance_to_day))
            .map_err(clock_in_repository_error)
    }

    async fn create_clock_in(&self, record: NewClockIn) -> Result<AttendanceDay, ClockInError> {
        validate_uuid(&record.user_id).map_err(clock_in_invalid_user_id)?;
        let mut transaction = self.pool.begin().await.map_err(clock_in_repository_error)?;
        let resolved_workday_id = lock_resolved_workday(
            &mut transaction,
            &record.resolved_workday_id,
            &record.user_id,
            record.work_date.as_naive_date(),
            record.recorded_at,
        )
        .await
        .map_err(clock_in_repository_error)?;
        let id = Uuid::new_v4().to_string();
        let attendance = sqlx::query_as::<_, AttendanceRow>(&format!(
            "INSERT INTO attendance \
             (id, user_id, date, resolved_workday_id, clock_in_time, clock_out_time, \
              status, total_work_hours, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, NULL, 'present', NULL, $6, $6) \
             RETURNING {}",
            ATTENDANCE_COLUMNS
        ))
        .bind(id)
        .bind(&record.user_id)
        .bind(record.work_date.as_naive_date())
        .bind(resolved_workday_id)
        .bind(record.clock_in_time)
        .bind(record.recorded_at)
        .fetch_one(&mut *transaction)
        .await
        .map_err(clock_in_repository_error)?;
        transaction
            .commit()
            .await
            .map_err(clock_in_repository_error)?;
        Ok(attendance_to_day(attendance))
    }

    async fn update_clock_in(
        &self,
        record: ExistingClockIn,
    ) -> Result<AttendanceDay, ClockInError> {
        validate_uuid(&record.attendance_id).map_err(clock_in_invalid_attendance_id)?;
        let mut transaction = self.pool.begin().await.map_err(clock_in_repository_error)?;
        let resolved_workday_id = lock_resolved_workday(
            &mut transaction,
            &record.resolved_workday_id,
            &record.user_id,
            record.work_date.as_naive_date(),
            record.recorded_at,
        )
        .await
        .map_err(clock_in_repository_error)?;
        let attendance = sqlx::query_as::<_, AttendanceRow>(&format!(
            "UPDATE attendance SET resolved_workday_id = $2, clock_in_time = $3, updated_at = $4 \
             WHERE id = $1 RETURNING {}",
            ATTENDANCE_COLUMNS
        ))
        .bind(record.attendance_id)
        .bind(resolved_workday_id)
        .bind(record.clock_in_time)
        .bind(record.recorded_at)
        .fetch_one(&mut *transaction)
        .await
        .map_err(clock_in_repository_error)?;
        transaction
            .commit()
            .await
            .map_err(clock_in_repository_error)?;
        Ok(attendance_to_day(attendance))
    }
}

#[async_trait]
impl ClockOutRepository for AttendanceWorkflowRepository {
    async fn find_for_clock_out(
        &self,
        user_id: &str,
        requested_work_date: Option<WorkDate>,
    ) -> Result<Option<AttendanceDay>, ClockOutError> {
        validate_uuid(user_id).map_err(clock_out_invalid_user_id)?;
        find_attendance_for_clock_out(
            &self.pool,
            user_id,
            requested_work_date.map(WorkDate::as_naive_date),
        )
        .await
        .map(|attendance| attendance.map(attendance_to_day))
        .map_err(clock_out_repository_error)
    }

    async fn has_active_break(&self, attendance_id: &str) -> Result<bool, ClockOutError> {
        validate_uuid(attendance_id).map_err(clock_out_invalid_attendance_id)?;
        has_active_break(&self.pool, attendance_id)
            .await
            .map_err(clock_out_repository_error)
    }

    async fn total_break_minutes(&self, attendance_id: &str) -> Result<i64, ClockOutError> {
        validate_uuid(attendance_id).map_err(clock_out_invalid_attendance_id)?;
        total_break_minutes(&self.pool, attendance_id)
            .await
            .map_err(clock_out_repository_error)
    }

    async fn update_clock_out(
        &self,
        record: ExistingClockOut,
    ) -> Result<AttendanceDay, ClockOutError> {
        validate_uuid(&record.attendance_id).map_err(clock_out_invalid_attendance_id)?;
        let attendance = sqlx::query_as::<_, AttendanceRow>(&format!(
            "UPDATE attendance SET clock_out_time = $2, total_work_hours = $3, updated_at = $4 \
             WHERE id = $1 RETURNING {}",
            ATTENDANCE_COLUMNS
        ))
        .bind(record.attendance_id)
        .bind(record.clock_out_time)
        .bind(record.total_work_hours)
        .bind(record.recorded_at)
        .fetch_one(&self.pool)
        .await
        .map_err(clock_out_repository_error)?;
        Ok(attendance_to_day(attendance))
    }
}

#[async_trait]
impl AttendanceStatusReadRepository for AttendanceWorkflowRepository {
    async fn find_by_user_and_date(
        &self,
        user_id: &str,
        work_date: WorkDate,
    ) -> Result<Option<AttendanceDay>, AttendanceStatusError> {
        validate_uuid(user_id).map_err(attendance_status_invalid_user_id)?;
        find_attendance_by_user_and_date(&self.pool, user_id, work_date.as_naive_date())
            .await
            .map(|attendance| attendance.map(attendance_to_day))
            .map_err(attendance_status_repository_error)
    }

    async fn active_break_id(
        &self,
        attendance_id: &str,
    ) -> Result<Option<String>, AttendanceStatusError> {
        validate_uuid(attendance_id).map_err(attendance_status_invalid_attendance_id)?;
        active_break_id(&self.pool, attendance_id)
            .await
            .map_err(attendance_status_repository_error)
    }
}

#[async_trait]
impl GetBreaksByAttendanceRepository for AttendanceWorkflowRepository {
    async fn find_attendance(
        &self,
        attendance_id: &str,
    ) -> Result<AttendanceDay, GetBreaksByAttendanceError> {
        validate_uuid(attendance_id).map_err(get_breaks_invalid_attendance_id)?;
        find_attendance_by_id(&self.pool, attendance_id)
            .await
            .map(attendance_to_day)
            .map_err(get_breaks_attendance_error)
    }

    async fn breaks_for_attendance(
        &self,
        attendance_id: &str,
    ) -> Result<Vec<BreakPeriod>, GetBreaksByAttendanceError> {
        validate_uuid(attendance_id).map_err(get_breaks_invalid_attendance_id)?;
        find_breaks_by_attendance_id(&self.pool, attendance_id)
            .await
            .map(|records| records.into_iter().map(break_record_to_period).collect())
            .map_err(get_breaks_repository_error)
    }
}

#[async_trait]
impl ListActiveBreaksRepository for AttendanceWorkflowRepository {
    async fn list_active_breaks(&self) -> Result<Vec<ActiveBreakSummary>, ListActiveBreaksError> {
        list_active_breaks(&self.pool)
            .await
            .map(|items| items.into_iter().map(active_break_to_summary).collect())
            .map_err(list_active_breaks_repository_error)
    }
}

#[async_trait]
impl ListAttendancePageRepository for AttendanceWorkflowRepository {
    async fn count_attendance(&self) -> Result<i64, ListAttendancePageError> {
        count_attendance(&self.pool)
            .await
            .map_err(list_attendance_page_repository_error)
    }

    async fn list_attendance(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AttendanceRecord>, ListAttendancePageError> {
        list_attendance(&self.pool, limit, offset)
            .await
            .map(|items| items.into_iter().map(attendance_record_to_app).collect())
            .map_err(list_attendance_page_repository_error)
    }

    async fn breaks_for_attendance_ids(
        &self,
        attendance_ids: &[String],
    ) -> Result<Vec<BreakPeriod>, ListAttendancePageError> {
        find_breaks_by_attendance_ids(&self.pool, attendance_ids)
            .await
            .map(|records| records.into_iter().map(break_record_to_period).collect())
            .map_err(list_attendance_page_repository_error)
    }
}

#[async_trait]
impl UpsertAttendanceRepository for AttendanceWorkflowRepository {
    async fn replace_attendance(
        &self,
        replacement: AttendanceReplacement,
    ) -> Result<timekeeper_app::attendance::AttendancePageItem, UpsertAttendanceError> {
        validate_uuid(&replacement.user_id).map_err(upsert_attendance_invalid_user_id)?;
        replace_attendance(&self.pool, replacement)
            .await
            .map_err(upsert_attendance_repository_error)
    }
}

#[async_trait]
impl ListUserAttendanceRepository for AttendanceWorkflowRepository {
    async fn list_user_attendance(
        &self,
        user_id: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<AttendanceRecord>, ListUserAttendanceError> {
        validate_uuid(user_id).map_err(list_user_attendance_invalid_user_id)?;
        list_user_attendance(&self.pool, user_id, from, to)
            .await
            .map(|items| items.into_iter().map(attendance_record_to_app).collect())
            .map_err(list_user_attendance_repository_error)
    }

    async fn breaks_for_attendance_ids(
        &self,
        attendance_ids: &[String],
    ) -> Result<Vec<BreakPeriod>, ListUserAttendanceError> {
        find_breaks_by_attendance_ids(&self.pool, attendance_ids)
            .await
            .map(|records| records.into_iter().map(break_record_to_period).collect())
            .map_err(list_user_attendance_repository_error)
    }

    async fn effective_corrections_for_attendance_ids(
        &self,
        attendance_ids: &[String],
    ) -> Result<Vec<EffectiveAttendanceCorrection>, ListUserAttendanceError> {
        find_effective_corrections_by_attendance_ids(&self.pool, attendance_ids)
            .await
            .map(|corrections| {
                corrections
                    .into_iter()
                    .map(effective_correction_to_app)
                    .collect()
            })
            .map_err(list_user_attendance_repository_error)
    }

    async fn list_user_attendance_with_optional_range(
        &self,
        user_id: &str,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
    ) -> Result<Vec<AttendanceRecord>, ListUserAttendanceError> {
        validate_uuid(user_id).map_err(list_user_attendance_invalid_user_id)?;
        list_user_attendance_with_optional_range(&self.pool, user_id, from, to)
            .await
            .map(|items| items.into_iter().map(attendance_record_to_app).collect())
            .map_err(list_user_attendance_repository_error)
    }

    async fn approved_leave_days(
        &self,
        user_id: &str,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
    ) -> Result<Vec<LeaveDayRecord>, ListUserAttendanceError> {
        validate_uuid(user_id).map_err(list_user_attendance_invalid_user_id)?;
        list_approved_leave_days(&self.pool, user_id, from, to)
            .await
            .map(|rows| rows.into_iter().map(leave_day_row_to_app).collect())
            .map_err(list_user_attendance_repository_error)
    }
}

#[async_trait]
impl StartBreakRepository for AttendanceWorkflowRepository {
    async fn find_attendance(&self, attendance_id: &str) -> Result<AttendanceDay, StartBreakError> {
        validate_uuid(attendance_id).map_err(start_break_invalid_attendance_id)?;
        find_attendance_by_id(&self.pool, attendance_id)
            .await
            .map(attendance_to_day)
            .map_err(start_break_attendance_error)
    }

    async fn has_active_break(&self, attendance_id: &str) -> Result<bool, StartBreakError> {
        validate_uuid(attendance_id).map_err(start_break_invalid_attendance_id)?;
        has_active_break(&self.pool, attendance_id)
            .await
            .map_err(start_break_repository_error)
    }

    async fn create_break(&self, record: NewBreakPeriod) -> Result<BreakPeriod, StartBreakError> {
        validate_uuid(&record.attendance_id).map_err(start_break_invalid_attendance_id)?;
        let id = Uuid::new_v4().to_string();
        let break_record = sqlx::query_as::<_, BreakRecordRow>(&format!(
            "INSERT INTO break_records \
             (id, attendance_id, break_start_time, break_end_time, duration_minutes, created_at, updated_at) \
             VALUES ($1, $2, $3, NULL, NULL, $4, $4) \
             RETURNING {}",
            BREAK_COLUMNS
        ))
        .bind(id)
        .bind(record.attendance_id)
        .bind(record.break_start_time)
        .bind(record.recorded_at)
        .fetch_one(&self.pool)
        .await
        .map_err(start_break_repository_error)?;
        Ok(break_record_to_period(break_record))
    }
}

#[async_trait]
impl BreakEndRepository for AttendanceWorkflowRepository {
    async fn find_break(&self, break_id: &str) -> Result<BreakPeriod, BreakEndError> {
        validate_uuid(break_id).map_err(break_end_invalid_break_id)?;
        find_break_by_id(&self.pool, break_id)
            .await
            .map(break_record_to_period)
            .map_err(break_end_break_error)
    }

    async fn find_attendance(&self, attendance_id: &str) -> Result<AttendanceDay, BreakEndError> {
        validate_uuid(attendance_id).map_err(break_end_invalid_attendance_id)?;
        find_attendance_by_id(&self.pool, attendance_id)
            .await
            .map(attendance_to_day)
            .map_err(break_end_attendance_error)
    }

    async fn update_break(&self, record: EndedBreakPeriod) -> Result<BreakPeriod, BreakEndError> {
        validate_uuid(&record.break_id).map_err(break_end_invalid_break_id)?;
        validate_uuid(&record.attendance_id).map_err(break_end_invalid_attendance_id)?;
        let break_record = sqlx::query_as::<_, BreakRecordRow>(&format!(
            "UPDATE break_records SET break_end_time = $2, duration_minutes = $3, updated_at = $4 \
             WHERE id = $1 RETURNING {}",
            BREAK_COLUMNS
        ))
        .bind(record.break_id)
        .bind(record.break_end_time)
        .bind(record.duration_minutes)
        .bind(record.recorded_at)
        .fetch_one(&self.pool)
        .await
        .map_err(break_end_repository_error)?;
        Ok(break_record_to_period(break_record))
    }

    async fn recalculate_total_hours(
        &self,
        attendance_id: &str,
        recorded_at: DateTime<Utc>,
    ) -> Result<(), BreakEndError> {
        validate_uuid(attendance_id).map_err(break_end_invalid_attendance_id)?;
        let attendance = find_attendance_by_id(&self.pool, attendance_id)
            .await
            .map_err(break_end_attendance_error)?;
        let (Some(clock_in_time), Some(clock_out_time)) =
            (attendance.clock_in_time, attendance.clock_out_time)
        else {
            return Ok(());
        };
        let break_minutes = total_break_minutes(&self.pool, attendance_id)
            .await
            .map_err(break_end_repository_error)?;
        let total_work_hours =
            calculate_total_work_hours(clock_in_time, clock_out_time, break_minutes);
        sqlx::query("UPDATE attendance SET total_work_hours = $2, updated_at = $3 WHERE id = $1")
            .bind(attendance_id)
            .bind(total_work_hours)
            .bind(recorded_at)
            .execute(&self.pool)
            .await
            .map_err(break_end_repository_error)?;
        Ok(())
    }
}

async fn find_attendance_by_user_and_date(
    pool: &PgPool,
    user_id: &str,
    date: NaiveDate,
) -> Result<Option<AttendanceRow>, sqlx::Error> {
    sqlx::query_as::<_, AttendanceRow>(&format!(
        "SELECT {} FROM attendance WHERE user_id = $1 AND date = $2",
        ATTENDANCE_COLUMNS
    ))
    .bind(user_id)
    .bind(date)
    .fetch_optional(pool)
    .await
}

async fn find_attendance_for_clock_out(
    pool: &PgPool,
    user_id: &str,
    requested_work_date: Option<NaiveDate>,
) -> Result<Option<AttendanceRow>, sqlx::Error> {
    if let Some(work_date) = requested_work_date {
        return find_attendance_by_user_and_date(pool, user_id, work_date).await;
    }
    sqlx::query_as::<_, AttendanceRow>(&format!(
        "SELECT {} FROM attendance \
         WHERE user_id = $1 AND clock_in_time IS NOT NULL AND clock_out_time IS NULL \
         ORDER BY date DESC, clock_in_time DESC LIMIT 1",
        ATTENDANCE_COLUMNS
    ))
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

async fn lock_resolved_workday(
    transaction: &mut sqlx::Transaction<'_, Postgres>,
    resolved_workday_id: &str,
    user_id: &str,
    work_date: NaiveDate,
    locked_at: DateTime<Utc>,
) -> Result<Uuid, sqlx::Error> {
    let resolved_workday_id = Uuid::parse_str(resolved_workday_id)
        .map_err(|_| sqlx::Error::Protocol("invalid resolved_workday_id".to_string()))?;
    let result = sqlx::query(
        "UPDATE resolved_workdays SET locked_at = $4 \
         WHERE id = $1 AND user_id = $2 AND work_date = $3 AND locked_at IS NULL",
    )
    .bind(resolved_workday_id)
    .bind(user_id)
    .bind(work_date)
    .bind(locked_at)
    .execute(&mut **transaction)
    .await?;
    if result.rows_affected() == 1 {
        return Ok(resolved_workday_id);
    }

    let already_locked: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM resolved_workdays \
         WHERE id = $1 AND user_id = $2 AND work_date = $3 AND locked_at IS NOT NULL)",
    )
    .bind(resolved_workday_id)
    .bind(user_id)
    .bind(work_date)
    .fetch_one(&mut **transaction)
    .await?;
    if already_locked {
        Ok(resolved_workday_id)
    } else {
        Err(sqlx::Error::Protocol(
            "resolved workday is unavailable for attendance".to_string(),
        ))
    }
}

async fn find_attendance_by_id(
    pool: &PgPool,
    attendance_id: &str,
) -> Result<AttendanceRow, sqlx::Error> {
    sqlx::query_as::<_, AttendanceRow>(&format!(
        "SELECT {} FROM attendance WHERE id = $1",
        ATTENDANCE_COLUMNS
    ))
    .bind(attendance_id)
    .fetch_optional(pool)
    .await?
    .ok_or(sqlx::Error::RowNotFound)
}

async fn find_break_by_id(pool: &PgPool, break_id: &str) -> Result<BreakRecordRow, sqlx::Error> {
    sqlx::query_as::<_, BreakRecordRow>(&format!(
        "SELECT {} FROM break_records WHERE id = $1",
        BREAK_COLUMNS
    ))
    .bind(break_id)
    .fetch_optional(pool)
    .await?
    .ok_or(sqlx::Error::RowNotFound)
}

async fn find_breaks_by_attendance_id(
    pool: &PgPool,
    attendance_id: &str,
) -> Result<Vec<BreakRecordRow>, sqlx::Error> {
    sqlx::query_as::<_, BreakRecordRow>(&format!(
        "SELECT {} FROM break_records
         WHERE attendance_id = $1
         ORDER BY break_start_time ASC",
        BREAK_COLUMNS
    ))
    .bind(attendance_id)
    .fetch_all(pool)
    .await
}

async fn list_active_breaks(pool: &PgPool) -> Result<Vec<ActiveBreakRow>, sqlx::Error> {
    sqlx::query_as::<_, ActiveBreakRow>(
        "SELECT
            br.id AS break_id,
            br.attendance_id,
            att.user_id,
            users.username,
            NULLIF(users.full_name_enc, '') AS full_name,
            br.break_start_time
         FROM break_records br
         INNER JOIN attendance att ON att.id = br.attendance_id
         INNER JOIN users ON users.id = att.user_id
         WHERE br.break_end_time IS NULL
         ORDER BY br.break_start_time DESC, users.username ASC
         LIMIT $1",
    )
    .bind(ACTIVE_BREAKS_LIST_LIMIT)
    .fetch_all(pool)
    .await
}

async fn count_attendance(pool: &PgPool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM attendance")
        .fetch_one(pool)
        .await
}

async fn list_attendance(
    pool: &PgPool,
    limit: i64,
    offset: i64,
) -> Result<Vec<AttendanceRecordRow>, sqlx::Error> {
    sqlx::query_as::<_, AttendanceRecordRow>(
        "SELECT id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours
         FROM attendance
         ORDER BY date DESC, user_id
         LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}

async fn list_user_attendance(
    pool: &PgPool,
    user_id: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<AttendanceRecordRow>, sqlx::Error> {
    sqlx::query_as::<_, AttendanceRecordRow>(
        "SELECT id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours
         FROM attendance
         WHERE user_id = $1 AND date BETWEEN $2 AND $3
         ORDER BY date DESC",
    )
    .bind(user_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await
}

async fn list_user_attendance_with_optional_range(
    pool: &PgPool,
    user_id: &str,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
) -> Result<Vec<AttendanceRecordRow>, sqlx::Error> {
    sqlx::query_as::<_, AttendanceRecordRow>(
        "SELECT id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours
         FROM attendance
         WHERE user_id = $1
           AND ($2::date IS NULL OR date >= $2)
           AND ($3::date IS NULL OR date <= $3)
         ORDER BY date DESC",
    )
    .bind(user_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await
}

async fn list_approved_leave_days(
    pool: &PgPool,
    user_id: &str,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
) -> Result<Vec<LeaveDayRowData>, sqlx::Error> {
    sqlx::query_as::<_, LeaveDayRowData>(
        "SELECT lr.id AS leave_request_id, d.day::date AS date, lr.leave_type
         FROM leave_requests lr
         CROSS JOIN LATERAL generate_series(
             lr.start_date::timestamp, lr.end_date::timestamp, interval '1 day'
         ) AS d(day)
         WHERE lr.user_id = $1
           AND lr.status = 'approved'
           AND ($2::date IS NULL OR d.day::date >= $2)
           AND ($3::date IS NULL OR d.day::date <= $3)
         ORDER BY d.day::date, lr.created_at, lr.id",
    )
    .bind(user_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await
}

async fn find_breaks_by_attendance_ids(
    pool: &PgPool,
    attendance_ids: &[String],
) -> Result<Vec<BreakRecordRow>, sqlx::Error> {
    if attendance_ids.is_empty() {
        return Ok(Vec::new());
    }

    sqlx::query_as::<_, BreakRecordRow>(&format!(
        "SELECT {} FROM break_records
         WHERE attendance_id = ANY($1)
         ORDER BY attendance_id ASC, break_start_time ASC",
        BREAK_COLUMNS
    ))
    .bind(attendance_ids)
    .fetch_all(pool)
    .await
}

async fn find_effective_corrections_by_attendance_ids(
    pool: &PgPool,
    attendance_ids: &[String],
) -> Result<Vec<EffectiveCorrectionRow>, sqlx::Error> {
    if attendance_ids.is_empty() {
        return Ok(Vec::new());
    }

    sqlx::query_as::<_, EffectiveCorrectionRow>(
        "SELECT attendance_id, clock_in_time_corrected, clock_out_time_corrected,
                break_records_corrected_json
         FROM attendance_correction_effective_values
         WHERE attendance_id = ANY($1)",
    )
    .bind(attendance_ids)
    .fetch_all(pool)
    .await
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

async fn list_admin_attendance_export(
    pool: &PgPool,
    filters: AdminAttendanceExportFilters,
) -> Result<Vec<AdminAttendanceExportRowData>, sqlx::Error> {
    let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT u.username,
                COALESCE(u.full_name_enc, '') AS full_name,
                a.date,
                a.clock_in_time,
                a.clock_out_time,
                a.total_work_hours,
                a.status
         FROM attendance a
         JOIN users u ON a.user_id = u.id",
    );
    let mut has_clause = false;
    push_where_clause(
        &mut builder,
        &mut has_clause,
        filters.username.as_ref(),
        |builder, value| {
            builder.push("u.username = ").push_bind(value);
        },
    );
    push_where_clause(
        &mut builder,
        &mut has_clause,
        filters.allowed_user_ids.as_ref(),
        |builder, value| {
            builder.push("u.id = ANY(").push_bind(value).push(")");
        },
    );
    push_where_clause(
        &mut builder,
        &mut has_clause,
        filters.from.as_ref(),
        |builder, value| {
            builder.push("a.date >= ").push_bind(value);
        },
    );
    push_where_clause(
        &mut builder,
        &mut has_clause,
        filters.to.as_ref(),
        |builder, value| {
            builder.push("a.date <= ").push_bind(value);
        },
    );
    builder.push(" ORDER BY a.date DESC, u.username");

    builder.build_query_as().fetch_all(pool).await
}

async fn list_admin_leave_days(
    pool: &PgPool,
    filters: AdminAttendanceExportFilters,
) -> Result<Vec<AdminLeaveDayRowData>, sqlx::Error> {
    let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT u.username,
                COALESCE(u.full_name_enc, '') AS full_name,
                d.day::date AS date,
                lr.leave_type
         FROM leave_requests lr
         JOIN users u ON lr.user_id = u.id
         CROSS JOIN LATERAL generate_series(
             lr.start_date::timestamp, lr.end_date::timestamp, interval '1 day'
         ) AS d(day)
         WHERE lr.status = 'approved'",
    );
    let mut has_clause = true;
    push_where_clause(
        &mut builder,
        &mut has_clause,
        filters.username.as_ref(),
        |builder, value| {
            builder.push("u.username = ").push_bind(value);
        },
    );
    push_where_clause(
        &mut builder,
        &mut has_clause,
        filters.allowed_user_ids.as_ref(),
        |builder, value| {
            builder.push("u.id = ANY(").push_bind(value).push(")");
        },
    );
    push_where_clause(
        &mut builder,
        &mut has_clause,
        filters.from.as_ref(),
        |builder, value| {
            builder.push("d.day::date >= ").push_bind(value);
        },
    );
    push_where_clause(
        &mut builder,
        &mut has_clause,
        filters.to.as_ref(),
        |builder, value| {
            builder.push("d.day::date <= ").push_bind(value);
        },
    );
    builder.push(" ORDER BY d.day::date DESC, u.username, lr.created_at, lr.id");

    builder.build_query_as().fetch_all(pool).await
}

async fn replace_attendance(
    pool: &PgPool,
    replacement: AttendanceReplacement,
) -> Result<timekeeper_app::attendance::AttendancePageItem, sqlx::Error> {
    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM attendance WHERE user_id = $1 AND date = $2")
        .bind(&replacement.user_id)
        .bind(replacement.date)
        .execute(&mut *tx)
        .await?;

    let attendance_id = Uuid::new_v4().to_string();
    let attendance = sqlx::query_as::<_, AttendanceRecordRow>(
        "INSERT INTO attendance
            (id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, 'present', $6, $7, $7)
         RETURNING id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours",
    )
    .bind(&attendance_id)
    .bind(&replacement.user_id)
    .bind(replacement.date)
    .bind(replacement.clock_in_time)
    .bind(replacement.clock_out_time)
    .bind(replacement.total_work_hours)
    .bind(replacement.recorded_at)
    .fetch_one(&mut *tx)
    .await?;

    let mut break_periods = Vec::with_capacity(replacement.breaks.len());
    for break_record in replacement.breaks {
        let row = sqlx::query_as::<_, BreakRecordRow>(&format!(
            "INSERT INTO break_records
                (id, attendance_id, break_start_time, break_end_time, duration_minutes, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $6)
             RETURNING {}",
            BREAK_COLUMNS
        ))
        .bind(Uuid::new_v4().to_string())
        .bind(&attendance_id)
        .bind(break_record.break_start_time)
        .bind(break_record.break_end_time)
        .bind(break_record.duration_minutes)
        .bind(replacement.recorded_at)
        .fetch_one(&mut *tx)
        .await?;
        break_periods.push(break_record_to_period(row));
    }

    tx.commit().await?;

    Ok(timekeeper_app::attendance::AttendancePageItem {
        attendance: attendance_record_to_app(attendance),
        break_periods,
    })
}

async fn has_active_break(pool: &PgPool, attendance_id: &str) -> Result<bool, sqlx::Error> {
    let active = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
            SELECT 1 FROM break_records
            WHERE attendance_id = $1 AND break_end_time IS NULL
        )",
    )
    .bind(attendance_id)
    .fetch_one(pool)
    .await?;
    Ok(active)
}

async fn active_break_id(
    pool: &PgPool,
    attendance_id: &str,
) -> Result<Option<String>, sqlx::Error> {
    let row = sqlx::query_scalar::<_, String>(
        "SELECT id FROM break_records
         WHERE attendance_id = $1 AND break_end_time IS NULL
         ORDER BY break_start_time DESC
         LIMIT 1",
    )
    .bind(attendance_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

async fn total_break_minutes(pool: &PgPool, attendance_id: &str) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        "SELECT COALESCE(SUM(duration_minutes), 0) AS minutes
         FROM break_records
         WHERE attendance_id = $1 AND duration_minutes IS NOT NULL",
    )
    .bind(attendance_id)
    .fetch_one(pool)
    .await?;
    Ok(row.try_get("minutes").unwrap_or(0))
}

fn attendance_to_day(attendance: AttendanceRow) -> AttendanceDay {
    AttendanceDay {
        attendance_id: attendance.id,
        user_id: attendance.user_id,
        work_date: WorkDate::from_naive_date(attendance.date),
        clock_in_time: attendance.clock_in_time,
        clock_out_time: attendance.clock_out_time,
    }
}

fn attendance_record_to_app(attendance: AttendanceRecordRow) -> AttendanceRecord {
    AttendanceRecord {
        attendance_id: attendance.id,
        user_id: attendance.user_id,
        date: attendance.date,
        clock_in_time: attendance.clock_in_time,
        clock_out_time: attendance.clock_out_time,
        status: attendance.status,
        total_work_hours: attendance.total_work_hours,
    }
}

fn break_record_to_period(record: BreakRecordRow) -> BreakPeriod {
    BreakPeriod {
        break_id: record.id,
        attendance_id: record.attendance_id,
        break_start_time: record.break_start_time,
        break_end_time: record.break_end_time,
        duration_minutes: record.duration_minutes,
    }
}

fn active_break_to_summary(row: ActiveBreakRow) -> ActiveBreakSummary {
    ActiveBreakSummary {
        break_id: row.break_id,
        attendance_id: row.attendance_id,
        user_id: row.user_id,
        username: row.username,
        full_name: row.full_name,
        break_start_time: row.break_start_time,
    }
}

fn effective_correction_to_app(row: EffectiveCorrectionRow) -> EffectiveAttendanceCorrection {
    let corrected_breaks =
        serde_json::from_value::<Vec<CorrectionBreakItemRow>>(row.break_records_corrected_json)
            .unwrap_or_default()
            .into_iter()
            .map(|break_item| {
                let duration_minutes = break_item.break_end_time.map(|break_end_time| {
                    break_end_time
                        .signed_duration_since(break_item.break_start_time)
                        .num_minutes()
                        .max(0) as i32
                });
                BreakPeriod {
                    break_id: Uuid::new_v4().to_string(),
                    attendance_id: row.attendance_id.clone(),
                    break_start_time: break_item.break_start_time,
                    break_end_time: break_item.break_end_time,
                    duration_minutes,
                }
            })
            .collect();

    EffectiveAttendanceCorrection {
        attendance_id: row.attendance_id,
        clock_in_time_corrected: row.clock_in_time_corrected,
        clock_out_time_corrected: row.clock_out_time_corrected,
        corrected_breaks,
    }
}

fn admin_export_row_to_app(row: AdminAttendanceExportRowData) -> AdminAttendanceExportRow {
    AdminAttendanceExportRow {
        username: row.username,
        full_name_encrypted: row.full_name,
        date: row.date,
        clock_in_time: row.clock_in_time,
        clock_out_time: row.clock_out_time,
        total_work_hours: row.total_work_hours,
        status: row.status,
        leave_type: None,
    }
}

fn leave_day_row_to_app(row: LeaveDayRowData) -> LeaveDayRecord {
    LeaveDayRecord {
        leave_request_id: row.leave_request_id,
        date: row.date,
        leave_type: row.leave_type,
    }
}

fn admin_leave_day_row_to_app(row: AdminLeaveDayRowData) -> AdminLeaveDayRow {
    AdminLeaveDayRow {
        username: row.username,
        full_name_encrypted: row.full_name,
        date: row.date,
        leave_type: row.leave_type,
    }
}

fn push_where_clause<'args, T, F>(
    builder: &mut QueryBuilder<'args, Postgres>,
    has_clause: &mut bool,
    value: Option<T>,
    push: F,
) where
    F: FnOnce(&mut QueryBuilder<'args, Postgres>, T),
{
    if let Some(value) = value {
        if *has_clause {
            builder.push(" AND ");
        } else {
            builder.push(" WHERE ");
            *has_clause = true;
        }
        push(builder, value);
    }
}

fn calculate_total_work_hours(
    clock_in_time: NaiveDateTime,
    clock_out_time: NaiveDateTime,
    break_minutes: i64,
) -> f64 {
    let gross_minutes = clock_out_time
        .signed_duration_since(clock_in_time)
        .num_minutes()
        .max(0);
    let net_minutes = gross_minutes - break_minutes.max(0);
    net_minutes.max(0) as f64 / 60.0
}

fn validate_uuid(value: &str) -> Result<(), uuid::Error> {
    Uuid::parse_str(value).map(|_| ())
}

fn clock_in_invalid_user_id(_: uuid::Error) -> ClockInError {
    ClockInError::Repository("invalid user_id".to_string())
}

fn clock_in_invalid_attendance_id(_: uuid::Error) -> ClockInError {
    ClockInError::Repository("invalid attendance_id".to_string())
}

fn clock_in_repository_error(error: sqlx::Error) -> ClockInError {
    ClockInError::Repository(error.to_string())
}

fn clock_out_invalid_user_id(_: uuid::Error) -> ClockOutError {
    ClockOutError::Repository("invalid user_id".to_string())
}

fn clock_out_invalid_attendance_id(_: uuid::Error) -> ClockOutError {
    ClockOutError::Repository("invalid attendance_id".to_string())
}

fn clock_out_repository_error(error: sqlx::Error) -> ClockOutError {
    match error {
        sqlx::Error::RowNotFound => ClockOutError::AttendanceNotFound,
        other => ClockOutError::Repository(other.to_string()),
    }
}

fn attendance_status_invalid_user_id(_: uuid::Error) -> AttendanceStatusError {
    AttendanceStatusError::Repository("invalid user_id".to_string())
}

fn attendance_status_invalid_attendance_id(_: uuid::Error) -> AttendanceStatusError {
    AttendanceStatusError::Repository("invalid attendance_id".to_string())
}

fn attendance_status_repository_error(error: sqlx::Error) -> AttendanceStatusError {
    AttendanceStatusError::Repository(error.to_string())
}

fn get_breaks_invalid_attendance_id(_: uuid::Error) -> GetBreaksByAttendanceError {
    GetBreaksByAttendanceError::Repository("invalid attendance_id".to_string())
}

fn get_breaks_attendance_error(error: sqlx::Error) -> GetBreaksByAttendanceError {
    match error {
        sqlx::Error::RowNotFound => GetBreaksByAttendanceError::AttendanceNotFound,
        other => GetBreaksByAttendanceError::Repository(other.to_string()),
    }
}

fn get_breaks_repository_error(error: sqlx::Error) -> GetBreaksByAttendanceError {
    GetBreaksByAttendanceError::Repository(error.to_string())
}

fn list_active_breaks_repository_error(error: sqlx::Error) -> ListActiveBreaksError {
    ListActiveBreaksError::Repository(error.to_string())
}

fn list_attendance_page_repository_error(error: sqlx::Error) -> ListAttendancePageError {
    ListAttendancePageError::Repository(error.to_string())
}

fn upsert_attendance_invalid_user_id(_: uuid::Error) -> UpsertAttendanceError {
    UpsertAttendanceError::Repository("invalid user_id".to_string())
}

fn upsert_attendance_repository_error(error: sqlx::Error) -> UpsertAttendanceError {
    UpsertAttendanceError::Repository(error.to_string())
}

fn list_user_attendance_invalid_user_id(_: uuid::Error) -> ListUserAttendanceError {
    ListUserAttendanceError::Repository("invalid user_id".to_string())
}

fn list_user_attendance_repository_error(error: sqlx::Error) -> ListUserAttendanceError {
    ListUserAttendanceError::Repository(error.to_string())
}

fn export_admin_invalid_user_id(_: uuid::Error) -> ExportAdminAttendanceError {
    ExportAdminAttendanceError::Repository("invalid user_id".to_string())
}

fn export_admin_repository_error(error: sqlx::Error) -> ExportAdminAttendanceError {
    ExportAdminAttendanceError::Repository(error.to_string())
}

fn start_break_invalid_attendance_id(_: uuid::Error) -> StartBreakError {
    StartBreakError::Repository("invalid attendance_id".to_string())
}

fn start_break_attendance_error(error: sqlx::Error) -> StartBreakError {
    match error {
        sqlx::Error::RowNotFound => StartBreakError::AttendanceNotFound,
        other => StartBreakError::Repository(other.to_string()),
    }
}

fn start_break_repository_error(error: sqlx::Error) -> StartBreakError {
    StartBreakError::Repository(error.to_string())
}

fn break_end_invalid_break_id(_: uuid::Error) -> BreakEndError {
    BreakEndError::Repository("invalid break_record_id".to_string())
}

fn break_end_invalid_attendance_id(_: uuid::Error) -> BreakEndError {
    BreakEndError::Repository("invalid attendance_id".to_string())
}

fn break_end_break_error(error: sqlx::Error) -> BreakEndError {
    match error {
        sqlx::Error::RowNotFound => BreakEndError::BreakNotFound,
        other => BreakEndError::Repository(other.to_string()),
    }
}

fn break_end_attendance_error(error: sqlx::Error) -> BreakEndError {
    match error {
        sqlx::Error::RowNotFound => BreakEndError::AttendanceNotFound,
        other => BreakEndError::Repository(other.to_string()),
    }
}

fn break_end_repository_error(error: sqlx::Error) -> BreakEndError {
    BreakEndError::Repository(error.to_string())
}
