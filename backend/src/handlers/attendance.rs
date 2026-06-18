use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use chrono::{Datelike, Duration, Months, NaiveDate, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use std::str::FromStr;
use std::sync::Arc;
use timekeeper_app::attendance::{
    AttendanceStatus as AppAttendanceStatus, AttendanceStatusError,
    AttendanceStatusQuery as AppAttendanceStatusQuery, BreakEnd as BreakEndUseCase,
    BreakEndCommand as AppBreakEndCommand, BreakEndError, BreakPeriod as UseCaseBreakPeriod,
    ClockIn as ClockInUseCase, ClockInCommand as AppClockInCommand, ClockInError,
    ClockOut as ClockOutUseCase, ClockOutCommand as AppClockOutCommand, ClockOutError,
    ExportUserAttendance as ExportUserAttendanceUseCase,
    ExportUserAttendanceQuery as AppExportUserAttendanceQuery,
    GetAttendanceStatus as GetAttendanceStatusUseCase,
    GetBreaksByAttendance as GetBreaksByAttendanceUseCase, GetBreaksByAttendanceError,
    GetBreaksByAttendanceQuery as AppGetBreaksByAttendanceQuery,
    GetUserAttendanceSummary as GetUserAttendanceSummaryUseCase,
    GetUserAttendanceSummaryQuery as AppGetUserAttendanceSummaryQuery,
    HolidayCalendar as ClockInHolidayCalendar, HolidayDecision as AppHolidayDecision,
    ListUserAttendance as ListUserAttendanceUseCase,
    ListUserAttendanceError as AppListUserAttendanceError,
    ListUserAttendanceQuery as AppListUserAttendanceQuery, StartBreak as StartBreakUseCase,
    StartBreakCommand as AppStartBreakCommand, StartBreakError, WorkdayCalendar,
};
use timekeeper_contract::attendance::AttendanceStatusResponse;
use timekeeper_domain::WorkDate;
use timekeeper_infra_postgres::attendance::AttendanceWorkflowRepository;
use utoipa::{IntoParams, ToSchema};

use crate::error::AppError;
use crate::handlers::attendance_utils::{fetch_attendance_by_user_date, get_break_records};
use crate::state::AppState;
use crate::types::{AttendanceId, BreakRecordId};
use crate::{
    models::{
        attendance::{
            Attendance, AttendanceResponse, AttendanceSummary, ClockInRequest, ClockOutRequest,
        },
        break_record::BreakRecordResponse,
        user::User,
    },
    services::holiday::HolidayServiceTrait,
    utils::{csv::render_user_attendance_export_csv, time},
};

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct AttendanceQuery {
    pub year: Option<i32>,
    pub month: Option<u32>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct AttendanceExportQuery {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

struct BackendClockInHolidayCalendar<'a> {
    service: &'a dyn HolidayServiceTrait,
}

#[async_trait::async_trait]
impl ClockInHolidayCalendar for BackendClockInHolidayCalendar<'_> {
    async fn decision_for(
        &self,
        user_id: &str,
        work_date: WorkDate,
    ) -> Result<AppHolidayDecision, ClockInError> {
        let decision = self
            .service
            .is_holiday(work_date.as_naive_date(), Some(user_id))
            .await
            .map_err(|error| ClockInError::HolidayCalendar(error.to_string()))?;
        if decision.is_holiday {
            Ok(AppHolidayDecision::Holiday {
                reason: decision.reason.label().to_string(),
            })
        } else {
            Ok(AppHolidayDecision::WorkingDay)
        }
    }
}

#[async_trait::async_trait]
impl WorkdayCalendar<ClockOutError> for BackendClockInHolidayCalendar<'_> {
    async fn decision_for(
        &self,
        user_id: &str,
        work_date: WorkDate,
    ) -> Result<AppHolidayDecision, ClockOutError> {
        let decision = self
            .service
            .is_holiday(work_date.as_naive_date(), Some(user_id))
            .await
            .map_err(|error| ClockOutError::HolidayCalendar(error.to_string()))?;
        if decision.is_holiday {
            Ok(AppHolidayDecision::Holiday {
                reason: decision.reason.label().to_string(),
            })
        } else {
            Ok(AppHolidayDecision::WorkingDay)
        }
    }
}

fn break_period_to_response(period: UseCaseBreakPeriod) -> BreakRecordResponse {
    BreakRecordResponse {
        id: period.break_id,
        attendance_id: period.attendance_id,
        break_start_time: period.break_start_time,
        break_end_time: period.break_end_time,
        duration_minutes: period.duration_minutes,
    }
}

fn break_periods_to_response(periods: Vec<UseCaseBreakPeriod>) -> Vec<BreakRecordResponse> {
    periods.into_iter().map(break_period_to_response).collect()
}

fn attendance_page_item_to_response(
    item: timekeeper_app::attendance::AttendancePageItem,
) -> AttendanceResponse {
    AttendanceResponse {
        id: item.attendance.attendance_id,
        user_id: item.attendance.user_id,
        date: item.attendance.date,
        clock_in_time: item.attendance.clock_in_time,
        clock_out_time: item.attendance.clock_out_time,
        status: item.attendance.status,
        total_work_hours: item.attendance.total_work_hours,
        break_records: break_periods_to_response(item.break_periods),
    }
}

fn user_attendance_summary_to_response(
    summary: timekeeper_app::attendance::UserAttendanceSummary,
) -> AttendanceSummary {
    AttendanceSummary {
        month: summary.month,
        year: summary.year,
        total_work_hours: summary.total_work_hours,
        total_work_days: summary.total_work_days,
        average_daily_hours: summary.average_daily_hours,
    }
}

fn attendance_status_to_response(status: AppAttendanceStatus) -> AttendanceStatusResponse {
    AttendanceStatusResponse {
        status: status.status,
        attendance_id: status.attendance_id,
        active_break_id: status.active_break_id,
        clock_in_time: status.clock_in_time,
        clock_out_time: status.clock_out_time,
    }
}

fn clock_in_error_to_app_error(error: ClockInError) -> AppError {
    match error {
        ClockInError::AlreadyClockedIn => AppError::BadRequest("Already clocked in today".into()),
        ClockInError::Holiday { work_date, reason } => AppError::Forbidden(format!(
            "{} is a {}. Submit an overtime request before clocking in/out.",
            work_date, reason
        )),
        ClockInError::Repository(message) | ClockInError::HolidayCalendar(message) => {
            AppError::InternalServerError(anyhow::anyhow!(message))
        }
    }
}

fn clock_out_error_to_app_error(error: ClockOutError) -> AppError {
    match error {
        ClockOutError::AttendanceNotFound => {
            AppError::NotFound("No attendance record found for today".into())
        }
        ClockOutError::ClockInRequired => {
            AppError::BadRequest("Must clock in before clocking out".into())
        }
        ClockOutError::AlreadyClockedOut => {
            AppError::BadRequest("Already clocked out today".into())
        }
        ClockOutError::ActiveBreakInProgress => {
            AppError::BadRequest("Break in progress. End break before clocking out".into())
        }
        ClockOutError::Holiday { work_date, reason } => AppError::Forbidden(format!(
            "{} is a {}. Submit an overtime request before clocking in/out.",
            work_date, reason
        )),
        ClockOutError::Repository(message) | ClockOutError::HolidayCalendar(message) => {
            AppError::InternalServerError(anyhow::anyhow!(message))
        }
    }
}

fn start_break_error_to_app_error(error: StartBreakError) -> AppError {
    match error {
        StartBreakError::AttendanceNotFound => {
            AppError::NotFound("Attendance record not found".into())
        }
        StartBreakError::Forbidden => AppError::Forbidden("Forbidden".into()),
        StartBreakError::ClockInRequired => {
            AppError::BadRequest("Must be clocked in to start break".into())
        }
        StartBreakError::ActiveBreakInProgress => {
            AppError::BadRequest("Break already in progress".into())
        }
        StartBreakError::Repository(message) => {
            AppError::InternalServerError(anyhow::anyhow!(message))
        }
    }
}

fn break_end_error_to_app_error(error: BreakEndError) -> AppError {
    match error {
        BreakEndError::BreakNotFound => AppError::NotFound("Break record not found".into()),
        BreakEndError::AttendanceNotFound => {
            AppError::NotFound("Attendance record not found".into())
        }
        BreakEndError::Forbidden => AppError::Forbidden("Forbidden".into()),
        BreakEndError::BreakAlreadyEnded => AppError::BadRequest("Break already ended".into()),
        BreakEndError::Repository(message) => {
            AppError::InternalServerError(anyhow::anyhow!(message))
        }
    }
}

fn attendance_status_error_to_app_error(error: AttendanceStatusError) -> AppError {
    match error {
        AttendanceStatusError::Repository(message) => {
            AppError::InternalServerError(anyhow::anyhow!(message))
        }
    }
}

fn get_breaks_error_to_app_error(error: GetBreaksByAttendanceError) -> AppError {
    match error {
        GetBreaksByAttendanceError::AttendanceNotFound => {
            AppError::NotFound("Attendance record not found".into())
        }
        GetBreaksByAttendanceError::Forbidden => AppError::Forbidden("Forbidden".into()),
        GetBreaksByAttendanceError::Repository(message) => {
            AppError::InternalServerError(anyhow::anyhow!(message))
        }
    }
}

fn list_user_attendance_error_to_app_error(error: AppListUserAttendanceError) -> AppError {
    match error {
        AppListUserAttendanceError::Repository(message) => {
            AppError::InternalServerError(anyhow::anyhow!(message))
        }
    }
}

pub async fn clock_in(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Extension(holiday_service): Extension<Arc<dyn HolidayServiceTrait>>,
    Json(payload): Json<ClockInRequest>,
) -> Result<Json<AttendanceResponse>, AppError> {
    let user_id = user.id;

    let tz = &state.config.time_zone;
    let now_local = time::now_in_timezone(tz);
    let now_utc = now_local.with_timezone(&Utc);
    let date = payload.date.unwrap_or_else(|| now_local.date_naive());
    let clock_in_time = now_local.naive_local();
    let work_date = WorkDate::from_naive_date(date);

    let use_case = ClockInUseCase::new(
        AttendanceWorkflowRepository::new(state.write_pool.clone()),
        BackendClockInHolidayCalendar {
            service: holiday_service.as_ref(),
        },
    );
    use_case
        .execute(AppClockInCommand {
            user_id: user_id.to_string(),
            work_date,
            clock_in_time,
            recorded_at: now_utc,
        })
        .await
        .map_err(clock_in_error_to_app_error)?;

    let attendance = fetch_attendance_by_user_date(&state.write_pool, user_id, date)
        .await?
        .ok_or_else(|| AppError::InternalServerError(anyhow::anyhow!("clock-in not persisted")))?;

    let break_records = get_break_records(&state.write_pool, attendance.id).await?;
    let response = build_attendance_response(attendance, break_records);

    Ok(Json(response))
}

pub async fn clock_out(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Extension(holiday_service): Extension<Arc<dyn HolidayServiceTrait>>,
    Json(payload): Json<ClockOutRequest>,
) -> Result<Json<AttendanceResponse>, AppError> {
    let user_id = user.id;

    let tz = &state.config.time_zone;
    let now_local = time::now_in_timezone(tz);
    let now_utc = now_local.with_timezone(&Utc);
    let date = payload.date.unwrap_or_else(|| now_local.date_naive());
    let clock_out_time = now_local.naive_local();
    let work_date = WorkDate::from_naive_date(date);

    let use_case = ClockOutUseCase::new(
        AttendanceWorkflowRepository::new(state.write_pool.clone()),
        BackendClockInHolidayCalendar {
            service: holiday_service.as_ref(),
        },
    );
    use_case
        .execute(AppClockOutCommand {
            user_id: user_id.to_string(),
            work_date,
            clock_out_time,
            recorded_at: now_utc,
        })
        .await
        .map_err(clock_out_error_to_app_error)?;

    let attendance = fetch_attendance_by_user_date(&state.write_pool, user_id, date)
        .await?
        .ok_or_else(|| AppError::InternalServerError(anyhow::anyhow!("clock-out not persisted")))?;
    let break_records = get_break_records(&state.write_pool, attendance.id).await?;
    let response = build_attendance_response(attendance, break_records);

    Ok(Json(response))
}

pub async fn break_start(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<crate::models::attendance::BreakStartRequest>,
) -> Result<Json<BreakRecordResponse>, AppError> {
    parse_attendance_id(&payload.attendance_id)?;
    let tz = &state.config.time_zone;
    let now_local = time::now_in_timezone(tz);
    let now_utc = now_local.with_timezone(&Utc);
    let break_start_time = now_local.naive_local();

    let use_case =
        StartBreakUseCase::new(AttendanceWorkflowRepository::new(state.write_pool.clone()));
    let break_period = use_case
        .execute(AppStartBreakCommand {
            user_id: user.id.to_string(),
            attendance_id: payload.attendance_id,
            break_start_time,
            recorded_at: now_utc,
        })
        .await
        .map_err(start_break_error_to_app_error)?;

    Ok(Json(break_period_to_response(break_period)))
}

pub async fn break_end(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<crate::models::attendance::BreakEndRequest>,
) -> Result<Json<BreakRecordResponse>, AppError> {
    parse_break_record_id(&payload.break_record_id)?;
    let tz = &state.config.time_zone;
    let now_local = time::now_in_timezone(tz);
    let now_utc = now_local.with_timezone(&Utc);
    let break_end_time = now_local.naive_local();

    let use_case =
        BreakEndUseCase::new(AttendanceWorkflowRepository::new(state.write_pool.clone()));
    let break_period = use_case
        .execute(AppBreakEndCommand {
            user_id: user.id.to_string(),
            break_id: payload.break_record_id,
            break_end_time,
            recorded_at: now_utc,
        })
        .await
        .map_err(break_end_error_to_app_error)?;

    Ok(Json(break_period_to_response(break_period)))
}

pub async fn get_my_attendance(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(params): Query<AttendanceQuery>,
) -> Result<Json<Vec<AttendanceResponse>>, AppError> {
    let user_id = user.id;

    let tz = &state.config.time_zone;
    let (from, to) = if let (Some(f), Some(t)) = (params.from, params.to) {
        if f > t {
            return Err(AppError::BadRequest("from must be <= to".into()));
        }
        (f, t)
    } else if params.from.is_some() || params.to.is_some() {
        let f = params.from.unwrap_or_else(|| time::today_local(tz));
        let t = params.to.unwrap_or_else(|| time::today_local(tz));
        if f > t {
            return Err(AppError::BadRequest("from must be <= to".into()));
        }
        (f, t)
    } else {
        let now_local = time::now_in_timezone(tz);
        let year = params.year.unwrap_or_else(|| now_local.year());
        let month = params.month.unwrap_or_else(|| now_local.month());
        let Some(first_day) = NaiveDate::from_ymd_opt(year, month, 1) else {
            return Err(AppError::BadRequest("Invalid year/month provided".into()));
        };
        let Some(last_day) = first_day
            .checked_add_months(Months::new(1))
            .and_then(|d| d.checked_sub_signed(Duration::days(1)))
        else {
            return Err(AppError::BadRequest("Invalid year/month provided".into()));
        };
        (first_day, last_day)
    };

    let use_case = ListUserAttendanceUseCase::new(AttendanceWorkflowRepository::new(
        state.read_pool().clone(),
    ));
    let responses = use_case
        .execute(AppListUserAttendanceQuery {
            user_id: user_id.to_string(),
            from,
            to,
        })
        .await
        .map_err(list_user_attendance_error_to_app_error)?
        .into_iter()
        .map(attendance_page_item_to_response)
        .collect();

    Ok(Json(responses))
}

pub async fn get_attendance_status(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<AttendanceStatusResponse>, AppError> {
    let user_id = user.id;
    let date = params
        .get("date")
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| time::today_local(&state.config.time_zone));

    let use_case = GetAttendanceStatusUseCase::new(AttendanceWorkflowRepository::new(
        state.read_pool().clone(),
    ));
    let status = use_case
        .execute(AppAttendanceStatusQuery {
            user_id: user_id.to_string(),
            work_date: WorkDate::from_naive_date(date),
        })
        .await
        .map_err(attendance_status_error_to_app_error)?;

    Ok(Json(attendance_status_to_response(status)))
}

pub async fn get_breaks_by_attendance(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(attendance_id): Path<String>,
) -> Result<Json<Vec<BreakRecordResponse>>, AppError> {
    AttendanceId::from_str(&attendance_id)
        .map_err(|_| AppError::BadRequest("Invalid attendance ID format".into()))?;
    let use_case = GetBreaksByAttendanceUseCase::new(AttendanceWorkflowRepository::new(
        state.read_pool().clone(),
    ));
    let breaks = use_case
        .execute(AppGetBreaksByAttendanceQuery {
            user_id: user.id.to_string(),
            attendance_id,
        })
        .await
        .map_err(get_breaks_error_to_app_error)?;
    Ok(Json(break_periods_to_response(breaks)))
}

pub async fn get_my_summary(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(params): Query<AttendanceQuery>,
) -> Result<Json<AttendanceSummary>, AppError> {
    let user_id = user.id;

    let now_local = time::now_in_timezone(&state.config.time_zone);
    let year = params.year.unwrap_or_else(|| now_local.year());
    let month = params.month.unwrap_or_else(|| now_local.month());

    let Some(first_day) = NaiveDate::from_ymd_opt(year, month, 1) else {
        return Err(AppError::BadRequest("Invalid year/month provided".into()));
    };
    let Some(last_day) = first_day
        .checked_add_months(Months::new(1))
        .and_then(|d| d.checked_sub_signed(Duration::days(1)))
    else {
        return Err(AppError::BadRequest("Invalid year/month provided".into()));
    };

    let use_case = GetUserAttendanceSummaryUseCase::new(AttendanceWorkflowRepository::new(
        state.read_pool().clone(),
    ));
    let summary = use_case
        .execute(AppGetUserAttendanceSummaryQuery {
            user_id: user_id.to_string(),
            year,
            month,
            from: first_day,
            to: last_day,
        })
        .await
        .map_err(list_user_attendance_error_to_app_error)?;

    Ok(Json(user_attendance_summary_to_response(summary)))
}

pub async fn export_my_attendance(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(params): Query<AttendanceExportQuery>,
) -> Result<Json<Value>, AppError> {
    let from = params.from;
    let to = params.to;

    if let (Some(f), Some(t)) = (from, to) {
        if f > t {
            return Err(AppError::BadRequest("from must be <= to".into()));
        }
    }

    let use_case = ExportUserAttendanceUseCase::new(AttendanceWorkflowRepository::new(
        state.read_pool().clone(),
    ));
    let export = use_case
        .execute(AppExportUserAttendanceQuery {
            user_id: user.id.to_string(),
            username: user.username.clone(),
            full_name: user.full_name.clone(),
            from,
            to,
        })
        .await
        .map_err(list_user_attendance_error_to_app_error)?;
    let csv_data = render_user_attendance_export_csv(&export.rows);

    Ok(Json(json!({
        "csv_data": csv_data,
        "filename": format!(
            "my_attendance_export_{}.csv",
            time::now_in_timezone(&state.config.time_zone).format("%Y%m%d_%H%M%S")
        )
    })))
}

pub(crate) fn build_attendance_response(
    attendance: Attendance,
    break_records: Vec<BreakRecordResponse>,
) -> AttendanceResponse {
    AttendanceResponse {
        id: attendance.id.to_string(),
        user_id: attendance.user_id.to_string(),
        date: attendance.date,
        clock_in_time: attendance.clock_in_time,
        clock_out_time: attendance.clock_out_time,
        status: attendance.status.db_value().to_string(),
        total_work_hours: attendance.total_work_hours,
        break_records,
    }
}

#[cfg(test)]
pub(crate) async fn recalculate_total_hours(
    pool: &sqlx::PgPool,
    mut attendance: Attendance,
    updated_at: chrono::DateTime<Utc>,
) -> Result<(), AppError> {
    if attendance.clock_in_time.is_none() || attendance.clock_out_time.is_none() {
        return Ok(());
    }

    let break_repo = crate::repositories::break_record::BreakRecordRepository::new();
    let break_minutes = break_repo.get_total_duration(pool, attendance.id).await?;

    attendance.calculate_work_hours(break_minutes);
    attendance.updated_at = updated_at;

    let att_repo = crate::repositories::attendance::AttendanceRepository::new();
    use crate::repositories::attendance::AttendanceRepositoryTrait;
    att_repo.update(pool, &attendance).await?;

    Ok(())
}

fn parse_attendance_id(value: &str) -> Result<AttendanceId, AppError> {
    AttendanceId::from_str(value)
        .map_err(|_| AppError::BadRequest("Invalid attendance_id".to_string()))
}

fn parse_break_record_id(value: &str) -> Result<BreakRecordId, AppError> {
    BreakRecordId::from_str(value)
        .map_err(|_| AppError::BadRequest("Invalid break_record_id".to_string()))
}

#[cfg(test)]
async fn reject_if_holiday(
    holiday_service: &dyn HolidayServiceTrait,
    date: NaiveDate,
    user_id: crate::types::UserId,
) -> Result<(), AppError> {
    let decision = holiday_service
        .is_holiday(date, Some(&user_id.to_string()))
        .await?;

    if decision.is_holiday {
        let reason = decision.reason.label();
        return Err(AppError::Forbidden(format!(
            "{} is a {}. Submit an overtime request before clocking in/out.",
            date, reason
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::holiday::{HolidayCalendarEntry, HolidayDecision, HolidayReason};
    use crate::types::{AttendanceId, UserId};
    use chrono::NaiveDate;
    use sqlx::postgres::PgPoolOptions;
    use std::sync::Arc;

    #[test]
    fn test_attendance_query_default_values() {
        let query = AttendanceQuery {
            year: None,
            month: None,
            from: None,
            to: None,
        };
        assert!(query.year.is_none());
        assert!(query.month.is_none());
        assert!(query.from.is_none());
        assert!(query.to.is_none());
    }

    #[test]
    fn test_attendance_query_with_values() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let query = AttendanceQuery {
            year: Some(2024),
            month: Some(1),
            from: Some(date),
            to: Some(date),
        };
        assert_eq!(query.year, Some(2024));
        assert_eq!(query.month, Some(1));
        assert_eq!(query.from, Some(date));
        assert_eq!(query.to, Some(date));
    }

    #[test]
    fn test_attendance_export_query_default_values() {
        let query = AttendanceExportQuery {
            from: None,
            to: None,
        };
        assert!(query.from.is_none());
        assert!(query.to.is_none());
    }

    #[test]
    fn test_attendance_status_response_structure() {
        let response = AttendanceStatusResponse {
            status: "clocked_in".to_string(),
            attendance_id: Some("test-id".to_string()),
            active_break_id: None,
            clock_in_time: None,
            clock_out_time: None,
        };
        assert_eq!(response.status, "clocked_in");
        assert!(response.attendance_id.is_some());
        assert!(response.active_break_id.is_none());
    }

    #[test]
    fn test_clock_in_request_structure() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 15);
        let request = ClockInRequest { date };
        assert_eq!(request.date, date);
    }

    #[test]
    fn test_clock_out_request_structure() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 15);
        let request = ClockOutRequest { date };
        assert_eq!(request.date, date);
    }

    #[test]
    fn parse_attendance_id_accepts_uuid_strings() {
        let id = AttendanceId::new();

        let parsed = parse_attendance_id(&id.to_string()).expect("valid attendance id");

        assert_eq!(parsed, id);
    }

    #[test]
    fn parse_break_record_id_rejects_invalid_strings() {
        let error = parse_break_record_id("not-a-uuid").expect_err("invalid break id");

        assert!(
            matches!(error, AppError::BadRequest(message) if message == "Invalid break_record_id")
        );
    }

    #[test]
    fn test_attendance_summary_structure() {
        let summary = AttendanceSummary {
            month: 1,
            year: 2024,
            total_work_hours: 160.5,
            total_work_days: 20,
            average_daily_hours: 8.0,
        };
        assert_eq!(summary.month, 1);
        assert_eq!(summary.year, 2024);
        assert_eq!(summary.total_work_hours, 160.5);
        assert_eq!(summary.total_work_days, 20);
        assert_eq!(summary.average_daily_hours, 8.0);
    }

    struct FixedHolidayService {
        decision: HolidayDecision,
    }

    #[async_trait::async_trait]
    impl crate::services::holiday::HolidayServiceTrait for FixedHolidayService {
        async fn is_holiday(
            &self,
            _date: NaiveDate,
            _user_id: Option<&str>,
        ) -> sqlx::Result<HolidayDecision> {
            Ok(self.decision.clone())
        }

        async fn list_month(
            &self,
            _year: i32,
            _month: u32,
            _user_id: Option<&str>,
        ) -> sqlx::Result<Vec<HolidayCalendarEntry>> {
            Ok(Vec::new())
        }
    }

    #[tokio::test]
    async fn reject_if_holiday_allows_working_day() {
        let service = Arc::new(FixedHolidayService {
            decision: HolidayDecision {
                is_holiday: false,
                reason: HolidayReason::None,
            },
        });
        let date = NaiveDate::from_ymd_opt(2026, 2, 4).expect("date");
        let user_id = UserId::new();

        let result = reject_if_holiday(service.as_ref(), date, user_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn reject_if_holiday_rejects_holiday_with_reason() {
        let service = Arc::new(FixedHolidayService {
            decision: HolidayDecision {
                is_holiday: true,
                reason: HolidayReason::PublicHoliday,
            },
        });
        let date = NaiveDate::from_ymd_opt(2026, 2, 11).expect("date");
        let user_id = UserId::new();

        let result = reject_if_holiday(service.as_ref(), date, user_id).await;
        let err = result.expect_err("holiday should be rejected");
        match err {
            AppError::Forbidden(message) => assert!(message.contains("public holiday")),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn build_attendance_response_keeps_core_fields() {
        let now = Utc::now();
        let attendance = Attendance {
            id: AttendanceId::new(),
            user_id: UserId::new(),
            date: NaiveDate::from_ymd_opt(2026, 2, 4).expect("date"),
            clock_in_time: Some(
                chrono::NaiveDateTime::parse_from_str("2026-02-04T09:00:00", "%Y-%m-%dT%H:%M:%S")
                    .expect("clock in"),
            ),
            clock_out_time: None,
            status: crate::models::attendance::AttendanceStatus::Present,
            total_work_hours: None,
            created_at: now,
            updated_at: now,
        };

        let response = build_attendance_response(attendance.clone(), Vec::new());
        assert_eq!(response.id, attendance.id.to_string());
        assert_eq!(response.user_id, attendance.user_id.to_string());
        assert_eq!(response.date, attendance.date);
        assert_eq!(response.clock_in_time, attendance.clock_in_time);
        assert_eq!(response.clock_out_time, attendance.clock_out_time);
        assert_eq!(response.status, "present");
        assert!(response.break_records.is_empty());
    }

    #[tokio::test]
    async fn recalculate_total_hours_returns_early_when_times_missing() {
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://127.0.0.1:15432/timekeeper")
            .expect("lazy pool");
        let now = Utc::now();

        let mut attendance = Attendance::new(
            UserId::new(),
            NaiveDate::from_ymd_opt(2026, 2, 4).expect("date"),
            now,
        );
        attendance.clock_in_time = None;
        attendance.clock_out_time = None;

        let result = recalculate_total_hours(&pool, attendance, now).await;
        assert!(result.is_ok());
    }
}
