use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use chrono::{Datelike, Duration, Months, NaiveDate, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use std::str::FromStr;
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
    ListUserAttendance as ListUserAttendanceUseCase,
    ListUserAttendanceError as AppListUserAttendanceError,
    ListUserAttendanceQuery as AppListUserAttendanceQuery, StartBreak as StartBreakUseCase,
    StartBreakCommand as AppStartBreakCommand, StartBreakError,
};
use timekeeper_app::attendance_classification::{
    ClassificationDay, ClassificationTotals, FlexPeriodStatus, GetMonthlyClassification,
    MonthlyClassification, MonthlyClassificationError,
    MonthlyClassificationQuery as AppMonthlyClassificationQuery,
};
use timekeeper_app::work_schedules::ResolveWorkday;
use timekeeper_contract::attendance::{
    AttendanceLeaveResponse, AttendanceStatusResponse, ClassificationTotalsResponse,
    DailyClassificationResponse, FlexPeriodClassificationResponse, FlexPeriodStatusResponse,
    MonthlyClassificationQueryParams, MonthlyClassificationResponse,
};
use timekeeper_contract::work_schedules::{
    ResolvedDayKind as ContractResolvedDayKind, WorkScheduleType as ContractWorkScheduleType,
};
use timekeeper_domain::WorkDate;
use timekeeper_infra_postgres::attendance::AttendanceWorkflowRepository;
use timekeeper_infra_postgres::attendance_classification::{
    ClassificationPostgresRepository, PostgresWorkdayMaterializer,
};
use timekeeper_infra_postgres::work_schedules::WorkdayResolverPostgresRepository;
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

fn user_attendance_day_to_response(
    day: timekeeper_app::attendance::UserAttendanceDay,
) -> AttendanceResponse {
    AttendanceResponse {
        id: day.attendance.attendance_id,
        user_id: day.attendance.user_id,
        date: day.attendance.date,
        clock_in_time: day.attendance.clock_in_time,
        clock_out_time: day.attendance.clock_out_time,
        status: day.attendance.status,
        total_work_hours: day.attendance.total_work_hours,
        break_records: break_periods_to_response(day.break_periods),
        leave: day.leave.map(|leave| AttendanceLeaveResponse {
            leave_request_id: leave.leave_request_id,
            leave_type: leave.leave_type,
        }),
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
        leave_days: summary.leave_days,
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

fn monthly_classification_to_response(
    result: MonthlyClassification,
) -> MonthlyClassificationResponse {
    match result {
        MonthlyClassification::Calculated(calculated) => {
            MonthlyClassificationResponse::Calculated {
                year: calculated.year,
                month: calculated.month,
                days: calculated
                    .days
                    .into_iter()
                    .map(classification_day_to_response)
                    .collect(),
                totals: classification_totals_to_response(calculated.totals),
                flex_period: flex_period_to_response(calculated.flex_period),
            }
        }
        MonthlyClassification::UnresolvedDays => MonthlyClassificationResponse::UnresolvedDays,
        MonthlyClassification::WorkRuleNotConfigured => {
            MonthlyClassificationResponse::WorkRuleNotConfigured
        }
    }
}

fn classification_day_to_response(day: ClassificationDay) -> DailyClassificationResponse {
    DailyClassificationResponse {
        work_date: day.work_date,
        day_kind: classification_day_kind_to_response(day.day_kind),
        schedule_type: classification_schedule_type_to_response(day.schedule_type),
        actual_minutes: day.actual_minutes,
        scheduled_minutes: day.scheduled_minutes,
        statutory_within_minutes: day.statutory_within_minutes,
        statutory_excess_minutes: day.statutory_excess_minutes,
        legal_holiday_minutes: day.legal_holiday_minutes,
        night_minutes: day.night_minutes,
        in_progress: day.in_progress,
        locked: day.locked,
    }
}

fn classification_totals_to_response(totals: ClassificationTotals) -> ClassificationTotalsResponse {
    ClassificationTotalsResponse {
        actual_minutes: totals.actual_minutes,
        scheduled_minutes: totals.scheduled_minutes,
        statutory_within_minutes: totals.statutory_within_minutes,
        statutory_excess_minutes: totals.statutory_excess_minutes,
        legal_holiday_minutes: totals.legal_holiday_minutes,
        night_minutes: totals.night_minutes,
    }
}

fn flex_period_to_response(status: FlexPeriodStatus) -> FlexPeriodStatusResponse {
    match status {
        FlexPeriodStatus::NotApplicable => FlexPeriodStatusResponse::NotApplicable,
        FlexPeriodStatus::UnresolvedDays => FlexPeriodStatusResponse::UnresolvedDays,
        FlexPeriodStatus::VersionMixed => FlexPeriodStatusResponse::VersionMixed,
        FlexPeriodStatus::NotConfigured => FlexPeriodStatusResponse::NotConfigured,
        FlexPeriodStatus::Calculated(result) => FlexPeriodStatusResponse::Calculated {
            result: FlexPeriodClassificationResponse {
                contracted_minutes: result.contracted_minutes,
                statutory_frame_minutes: result.statutory_frame_minutes,
                actual_minutes: result.actual_minutes,
                scheduled_minutes: result.scheduled_minutes,
                statutory_within_minutes: result.statutory_within_minutes,
                statutory_excess_minutes: result.statutory_excess_minutes,
            },
        },
    }
}

fn classification_day_kind_to_response(
    day_kind: timekeeper_app::work_schedules::ResolvedDayKind,
) -> ContractResolvedDayKind {
    match day_kind {
        timekeeper_app::work_schedules::ResolvedDayKind::ScheduledWorkday => {
            ContractResolvedDayKind::ScheduledWorkday
        }
        timekeeper_app::work_schedules::ResolvedDayKind::ScheduledNonWorkingDay => {
            ContractResolvedDayKind::ScheduledNonWorkingDay
        }
        timekeeper_app::work_schedules::ResolvedDayKind::PublicHoliday => {
            ContractResolvedDayKind::PublicHoliday
        }
    }
}

fn classification_schedule_type_to_response(
    schedule_type: timekeeper_app::work_schedules::ScheduleType,
) -> ContractWorkScheduleType {
    match schedule_type {
        timekeeper_app::work_schedules::ScheduleType::Fixed => ContractWorkScheduleType::Fixed,
        timekeeper_app::work_schedules::ScheduleType::Flex => ContractWorkScheduleType::Flex,
    }
}

fn classification_error_to_app_error(error: MonthlyClassificationError) -> AppError {
    match error {
        MonthlyClassificationError::InvalidYear | MonthlyClassificationError::InvalidMonth => {
            AppError::BadRequestWithCode {
                message: error.to_string(),
                code: "INVALID_ATTENDANCE_CLASSIFICATION_QUERY".to_string(),
            }
        }
        MonthlyClassificationError::Repository(message) => {
            AppError::InternalServerError(anyhow::anyhow!(message))
        }
    }
}

pub async fn get_my_classification(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(params): Query<MonthlyClassificationQueryParams>,
) -> Result<Json<MonthlyClassificationResponse>, AppError> {
    monthly_classification_response(&state, &user.id.to_string(), params).await
}

pub(crate) async fn monthly_classification_response(
    state: &AppState,
    user_id: &str,
    params: MonthlyClassificationQueryParams,
) -> Result<Json<MonthlyClassificationResponse>, AppError> {
    let use_case = GetMonthlyClassification::new(
        ClassificationPostgresRepository::new(state.read_pool().clone()),
        PostgresWorkdayMaterializer::new(state.write_pool.clone()),
    );
    let result = use_case
        .execute(AppMonthlyClassificationQuery {
            user_id: user_id.to_string(),
            year: params.year,
            month: params.month,
        })
        .await
        .map_err(classification_error_to_app_error)?;
    Ok(Json(monthly_classification_to_response(result)))
}

fn clock_in_error_to_app_error(error: ClockInError) -> AppError {
    match error {
        ClockInError::AlreadyClockedIn => AppError::BadRequest("Already clocked in today".into()),
        ClockInError::WorkScheduleNotConfigured => AppError::UnprocessableEntityWithCode {
            message: "Work schedule is not configured for the requested day".to_string(),
            code: "WORK_SCHEDULE_NOT_CONFIGURED".to_string(),
        },
        ClockInError::Repository(message) | ClockInError::WorkdayResolution(message) => {
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
        ClockOutError::Repository(message) => {
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
    Json(payload): Json<ClockInRequest>,
) -> Result<Json<AttendanceResponse>, AppError> {
    let user_id = user.id;

    let tz = &state.config.time_zone;
    let now_local = time::now_in_timezone(tz);
    let now_utc = now_local.with_timezone(&Utc);
    let clock_in_time = now_local.naive_local();
    let resolver_repository = WorkdayResolverPostgresRepository::new(state.write_pool.clone());
    let workday_resolver = ResolveWorkday::new(
        resolver_repository.clone(),
        resolver_repository.clone(),
        resolver_repository,
    );

    let use_case = ClockInUseCase::new(
        AttendanceWorkflowRepository::new(state.write_pool.clone()),
        workday_resolver,
    );
    let attendance_day = use_case
        .execute(AppClockInCommand {
            user_id: user_id.to_string(),
            requested_work_date: payload.date.map(WorkDate::from_naive_date),
            clock_in_time,
            recorded_at: now_utc,
        })
        .await
        .map_err(clock_in_error_to_app_error)?;

    let attendance = fetch_attendance_by_user_date(
        &state.write_pool,
        user_id,
        attendance_day.work_date.as_naive_date(),
    )
    .await?
    .ok_or_else(|| AppError::InternalServerError(anyhow::anyhow!("clock-in not persisted")))?;

    let break_records = get_break_records(&state.write_pool, attendance.id).await?;
    let response = build_attendance_response(attendance, break_records);

    Ok(Json(response))
}

pub async fn clock_out(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<ClockOutRequest>,
) -> Result<Json<AttendanceResponse>, AppError> {
    let user_id = user.id;

    let tz = &state.config.time_zone;
    let now_local = time::now_in_timezone(tz);
    let now_utc = now_local.with_timezone(&Utc);
    let clock_out_time = now_local.naive_local();

    let use_case =
        ClockOutUseCase::new(AttendanceWorkflowRepository::new(state.write_pool.clone()));
    let attendance_day = use_case
        .execute(AppClockOutCommand {
            user_id: user_id.to_string(),
            requested_work_date: payload.date.map(WorkDate::from_naive_date),
            clock_out_time,
            recorded_at: now_utc,
        })
        .await
        .map_err(clock_out_error_to_app_error)?;

    let attendance = fetch_attendance_by_user_date(
        &state.write_pool,
        user_id,
        attendance_day.work_date.as_naive_date(),
    )
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
        .map(user_attendance_day_to_response)
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
        leave: None,
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
mod tests {
    use super::*;
    use crate::types::{AttendanceId, UserId};
    use chrono::NaiveDate;
    use sqlx::postgres::PgPoolOptions;

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
            leave_days: 0,
        };
        assert_eq!(summary.month, 1);
        assert_eq!(summary.year, 2024);
        assert_eq!(summary.total_work_hours, 160.5);
        assert_eq!(summary.total_work_days, 20);
        assert_eq!(summary.average_daily_hours, 8.0);
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
