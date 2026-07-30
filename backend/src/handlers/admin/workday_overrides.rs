use std::str::FromStr;

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::NaiveDate;
use timekeeper_app::work_schedules::WorkdayOverrideKind;
use timekeeper_app::workday_overrides::{
    DeleteWorkdayOverride, SetWorkdayOverride, SetWorkdayOverrideCommand, StoredWorkdayOverride,
    WorkdayOverrideError,
};
use timekeeper_contract::{
    attendance::{MonthlyClassificationQueryParams, MonthlyClassificationResponse},
    settlement_balance::{SettlementBalanceQueryParams, SettlementBalanceResponse},
    work_schedules::{
        ResolvedWorkdayListResponse, ResolvedWorkdayRangeQuery, SetWorkdayOverrideRequest,
        WorkdayOverrideKind as ContractWorkdayOverrideKind, WorkdayOverrideResponse,
    },
};
use timekeeper_infra_postgres::work_schedules::WorkdayResolverPostgresRepository;

use crate::{
    error::AppError, handlers::work_schedules::list_resolved_workdays, models::user::User,
    repositories::department::can_manager_approve, state::AppState, types::UserId,
};

pub async fn get_user_resolved_workdays(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(user_id): Path<String>,
    Query(query): Query<ResolvedWorkdayRangeQuery>,
) -> Result<Json<ResolvedWorkdayListResponse>, AppError> {
    let target = parse_user_id(&user_id)?;
    authorize_scope(&state, &user, target).await?;
    list_resolved_workdays(&state, &user_id, query).await
}

pub async fn get_user_classification(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(user_id): Path<String>,
    Query(query): Query<MonthlyClassificationQueryParams>,
) -> Result<Json<MonthlyClassificationResponse>, AppError> {
    let target = parse_user_id(&user_id)?;
    authorize_scope(&state, &user, target).await?;
    crate::handlers::attendance::monthly_classification_response(&state, &user_id, query).await
}

pub async fn get_user_settlement_balance(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(user_id): Path<String>,
    Query(query): Query<SettlementBalanceQueryParams>,
) -> Result<Json<SettlementBalanceResponse>, AppError> {
    let target = parse_user_id(&user_id)?;
    authorize_scope(&state, &user, target).await?;
    crate::handlers::work_schedules::settlement_balance_response(&state, &user_id, query).await
}

pub async fn set_workday_override(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path((user_id, date)): Path<(String, String)>,
    Json(payload): Json<SetWorkdayOverrideRequest>,
) -> Result<Json<WorkdayOverrideResponse>, AppError> {
    let target = parse_user_id(&user_id)?;
    authorize_scope(&state, &user, target).await?;
    let work_date = parse_date(&date)?;

    let repository = WorkdayResolverPostgresRepository::new(state.write_pool.clone());
    let use_case = SetWorkdayOverride::new(repository);
    let stored = use_case
        .execute(SetWorkdayOverrideCommand {
            user_id: user_id.clone(),
            work_date,
            kind: kind_from_request(payload.kind),
            work_schedule_id: payload.work_schedule_id,
            reason: payload.reason,
            created_by: user.id.to_string(),
        })
        .await
        .map_err(map_override_error)?;
    Ok(Json(stored_to_response(stored)))
}

pub async fn delete_workday_override(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path((user_id, date)): Path<(String, String)>,
) -> Result<StatusCode, AppError> {
    let target = parse_user_id(&user_id)?;
    authorize_scope(&state, &user, target).await?;
    let work_date = parse_date(&date)?;

    let repository = WorkdayResolverPostgresRepository::new(state.write_pool.clone());
    let use_case = DeleteWorkdayOverride::new(repository);
    use_case
        .execute(&user_id, work_date)
        .await
        .map_err(map_override_error)?;
    Ok(StatusCode::NO_CONTENT)
}

/// system admin は任意ユーザー、manager は配下ユーザーのみ操作できる。
async fn authorize_scope(state: &AppState, actor: &User, target: UserId) -> Result<(), AppError> {
    if actor.is_system_admin() {
        return Ok(());
    }
    if actor.is_manager()
        && can_manager_approve(state.read_pool(), actor.id, target)
            .await
            .map_err(|error| AppError::InternalServerError(error.into()))?
    {
        return Ok(());
    }
    Err(forbidden())
}

fn parse_user_id(value: &str) -> Result<UserId, AppError> {
    UserId::from_str(value).map_err(|_| invalid_work_schedule("invalid user_id"))
}

fn parse_date(value: &str) -> Result<NaiveDate, AppError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| invalid_work_schedule("date must be formatted as YYYY-MM-DD"))
}

fn kind_from_request(kind: ContractWorkdayOverrideKind) -> WorkdayOverrideKind {
    match kind {
        ContractWorkdayOverrideKind::NonWorkingDay => WorkdayOverrideKind::NonWorkingDay,
        ContractWorkdayOverrideKind::UseSchedule => WorkdayOverrideKind::UseSchedule,
    }
}

fn kind_to_response(kind: WorkdayOverrideKind) -> ContractWorkdayOverrideKind {
    match kind {
        WorkdayOverrideKind::NonWorkingDay => ContractWorkdayOverrideKind::NonWorkingDay,
        WorkdayOverrideKind::UseSchedule => ContractWorkdayOverrideKind::UseSchedule,
    }
}

fn stored_to_response(stored: StoredWorkdayOverride) -> WorkdayOverrideResponse {
    WorkdayOverrideResponse {
        id: stored.id,
        user_id: stored.user_id,
        work_date: stored.work_date,
        kind: kind_to_response(stored.kind),
        work_schedule_id: stored.work_schedule_id,
        reason: stored.reason,
        created_by: stored.created_by,
        created_at: stored.created_at,
        updated_at: stored.updated_at,
    }
}

fn map_override_error(error: WorkdayOverrideError) -> AppError {
    match error {
        WorkdayOverrideError::InvalidInput(message) => AppError::BadRequestWithCode {
            message,
            code: "INVALID_WORK_SCHEDULE".to_string(),
        },
        WorkdayOverrideError::NotFound => {
            AppError::NotFound("Workday override not found".to_string())
        }
        WorkdayOverrideError::ResolvedWorkdayLocked => AppError::ConflictWithCode {
            message: "Resolved workday is locked".to_string(),
            code: "RESOLVED_WORKDAY_LOCKED".to_string(),
        },
        WorkdayOverrideError::Repository(message) => {
            AppError::InternalServerError(anyhow::anyhow!(message))
        }
    }
}

fn invalid_work_schedule(message: &str) -> AppError {
    AppError::BadRequestWithCode {
        message: message.to_string(),
        code: "INVALID_WORK_SCHEDULE".to_string(),
    }
}

fn forbidden() -> AppError {
    AppError::Forbidden("Forbidden".to_string())
}
