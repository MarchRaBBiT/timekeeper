use crate::error::AppError;
use crate::models::{PaginatedResponse, PaginationQuery};
use crate::types::UserId;
use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use chrono::Utc;
use std::str::FromStr;
use timekeeper_app::attendance::{
    ActiveBreakSummary as AppActiveBreakSummary, BreakEndError, BreakPeriod as UseCaseBreakPeriod,
    ForceEndBreak as ForceEndBreakUseCase, ForceEndBreakCommand as AppForceEndBreakCommand,
    ListActiveBreaks as ListActiveBreaksUseCase, ListActiveBreaksError,
    ListAttendancePage as ListAttendancePageUseCase,
    ListAttendancePageError as AppListAttendancePageError,
    ListAttendancePageQuery as AppListAttendancePageQuery,
    UpsertAttendance as UpsertAttendanceUseCase,
    UpsertAttendanceCommand as AppUpsertAttendanceCommand,
    UpsertAttendanceError as AppUpsertAttendanceError, UpsertBreakInput as AppUpsertBreakInput,
};
use timekeeper_contract::attendance::AdminAttendanceUpsert;
use timekeeper_infra_postgres::attendance::AttendanceWorkflowRepository;

use crate::state::AppState;
use crate::{
    models::{attendance::AttendanceResponse, user::User},
    utils::{encryption::decrypt_pii, time},
};

pub async fn get_all_attendance(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<PaginatedResponse<AttendanceResponse>>, AppError> {
    if !user.is_system_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    let limit = pagination.limit();
    let offset = pagination.offset();

    let use_case = ListAttendancePageUseCase::new(AttendanceWorkflowRepository::new(
        state.read_pool().clone(),
    ));
    let page = use_case
        .execute(AppListAttendancePageQuery { limit, offset })
        .await
        .map_err(list_attendance_page_error_to_app_error)?;
    let data = page
        .items
        .into_iter()
        .map(attendance_page_item_to_response)
        .collect();

    Ok(Json(PaginatedResponse::new(
        data,
        page.total,
        page.limit,
        page.offset,
    )))
}

pub async fn upsert_attendance(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(body): Json<AdminAttendanceUpsert>,
) -> Result<Json<AttendanceResponse>, AppError> {
    if !user.is_system_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }
    use chrono::{NaiveDate, NaiveDateTime};

    let AdminAttendanceUpsert {
        user_id,
        date,
        clock_in_time,
        clock_out_time,
        breaks,
    } = body;

    let date = NaiveDate::parse_from_str(&date, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("Invalid date".into()))?;
    let cin = NaiveDateTime::parse_from_str(&clock_in_time, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(&clock_in_time, "%Y-%m-%d %H:%M:%S"))
        .map_err(|_| AppError::BadRequest("Invalid clock_in_time".into()))?;
    let cout = match &clock_out_time {
        Some(s) => Some(
            NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
                .map_err(|_| AppError::BadRequest("Invalid clock_out_time".into()))?,
        ),
        None => None,
    };

    // Parse and validate user_id
    let user_id_typed = UserId::from_str(&user_id)
        .map_err(|_| AppError::BadRequest("Invalid user_id format".into()))?;

    let breaks = breaks
        .unwrap_or_default()
        .into_iter()
        .map(|break_item| {
            let break_start_time = chrono::NaiveDateTime::parse_from_str(
                &break_item.break_start_time,
                "%Y-%m-%dT%H:%M:%S",
            )
            .or_else(|_| {
                chrono::NaiveDateTime::parse_from_str(
                    &break_item.break_start_time,
                    "%Y-%m-%d %H:%M:%S",
                )
            })
            .map_err(|_| AppError::BadRequest("Invalid break_start_time".into()))?;
            let break_end_time = break_item.break_end_time.as_ref().and_then(|value| {
                chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S")
                    .ok()
                    .or_else(|| {
                        chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S").ok()
                    })
            });
            Ok(AppUpsertBreakInput {
                break_start_time,
                break_end_time,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    let use_case =
        UpsertAttendanceUseCase::new(AttendanceWorkflowRepository::new(state.write_pool.clone()));
    let item = use_case
        .execute(AppUpsertAttendanceCommand {
            user_id: user_id_typed.to_string(),
            date,
            clock_in_time: cin,
            clock_out_time: cout,
            breaks,
            recorded_at: time::now_utc(&state.config.time_zone),
        })
        .await
        .map_err(upsert_attendance_error_to_app_error)?;

    Ok(Json(attendance_page_item_to_response(item)))
}

pub async fn list_active_breaks(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<crate::models::break_record::ActiveBreakResponse>>, AppError> {
    if !user.is_system_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    let use_case =
        ListActiveBreaksUseCase::new(AttendanceWorkflowRepository::new(state.read_pool().clone()));
    let active_breaks = use_case
        .execute()
        .await
        .map_err(list_active_breaks_error_to_app_error)?;

    let active_breaks = active_breaks
        .into_iter()
        .map(|item| active_break_to_response(item, &state.config))
        .collect::<Result<Vec<_>, AppError>>()?;

    Ok(Json(active_breaks))
}

// Admin: force end a break
// Admin: force end a break
pub async fn force_end_break(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(break_id): Path<String>,
) -> Result<Json<crate::models::break_record::BreakRecordResponse>, AppError> {
    if !user.is_system_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }
    crate::types::BreakRecordId::from_str(&break_id)
        .map_err(|_| AppError::BadRequest("Invalid break record ID format".into()))?;
    let now_local = time::now_in_timezone(&state.config.time_zone);
    let now_utc = now_local.with_timezone(&Utc);
    let now = now_local.naive_local();

    let use_case =
        ForceEndBreakUseCase::new(AttendanceWorkflowRepository::new(state.write_pool.clone()));
    let break_period = use_case
        .execute(AppForceEndBreakCommand {
            break_id,
            break_end_time: now,
            recorded_at: now_utc,
        })
        .await
        .map_err(force_end_break_error_to_app_error)?;

    Ok(Json(break_period_to_response(break_period)))
}

fn force_end_break_error_to_app_error(error: BreakEndError) -> AppError {
    match error {
        BreakEndError::BreakNotFound => AppError::NotFound("Break record not found".into()),
        BreakEndError::AttendanceNotFound => {
            AppError::NotFound("Attendance record not found".into())
        }
        BreakEndError::BreakAlreadyEnded => AppError::BadRequest("Break already ended".into()),
        BreakEndError::Forbidden => AppError::Forbidden("Forbidden".into()),
        BreakEndError::Repository(message) => {
            AppError::InternalServerError(anyhow::anyhow!(message))
        }
    }
}

fn list_active_breaks_error_to_app_error(error: ListActiveBreaksError) -> AppError {
    match error {
        ListActiveBreaksError::Repository(message) => {
            AppError::InternalServerError(anyhow::anyhow!(message))
        }
    }
}

fn list_attendance_page_error_to_app_error(error: AppListAttendancePageError) -> AppError {
    match error {
        AppListAttendancePageError::Repository(message) => {
            AppError::InternalServerError(anyhow::anyhow!(message))
        }
    }
}

fn upsert_attendance_error_to_app_error(error: AppUpsertAttendanceError) -> AppError {
    match error {
        AppUpsertAttendanceError::Repository(message) => {
            AppError::InternalServerError(anyhow::anyhow!(message))
        }
    }
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
        break_records: item
            .break_periods
            .into_iter()
            .map(break_period_to_response)
            .collect(),
        leave: None,
    }
}

fn break_period_to_response(
    period: UseCaseBreakPeriod,
) -> crate::models::break_record::BreakRecordResponse {
    crate::models::break_record::BreakRecordResponse {
        id: period.break_id,
        attendance_id: period.attendance_id,
        break_start_time: period.break_start_time,
        break_end_time: period.break_end_time,
        duration_minutes: period.duration_minutes,
    }
}

fn active_break_to_response(
    item: AppActiveBreakSummary,
    config: &crate::config::Config,
) -> Result<crate::models::break_record::ActiveBreakResponse, AppError> {
    Ok(crate::models::break_record::ActiveBreakResponse {
        break_id: item.break_id,
        attendance_id: item.attendance_id,
        user_id: item.user_id,
        username: item.username,
        full_name: item
            .full_name
            .as_ref()
            .map(|encrypted| decrypt_pii(encrypted, config).unwrap_or_else(|_| "***".to_string())),
        break_start_time: item.break_start_time,
    })
}
