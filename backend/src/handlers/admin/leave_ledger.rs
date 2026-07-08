use std::str::FromStr;

use axum::{
    extract::{Path, Query},
    Extension, Json,
};
use chrono::Utc;
use timekeeper_app::leave_ledger::{
    AdjustLeaveLedger, AdjustLeaveLedgerCommand, GetLeaveBalance, GetLeaveBalanceCommand,
    GrantSkipReason, RunLeaveGrants, RunLeaveGrantsCommand, SetHireDate,
};
use timekeeper_contract::leave::{
    HireDateResponse, LeaveBalanceQuery, LeaveBalanceResponse, LeaveExpiryBackfillResponse,
    LeaveGrantResultResponse, LeaveGrantRunRequest, LeaveGrantRunResponse, LeaveGrantSkipReason,
    LeaveGrantSkipResponse, LeaveLedgerAdjustRequest, LeaveLedgerAdjustResponse,
    SetHireDateRequest,
};
use timekeeper_infra_postgres::leave_ledger::LeaveLedgerPostgresRepository;
use validator::Validate;

use crate::{
    error::AppError,
    handlers::leave_ledger::{
        balance_view_to_response, entry_to_response, leave_ledger_error_to_app_error,
        minutes_to_days,
    },
    models::user::User,
    repositories::department::can_manager_approve,
    state::AppState,
    types::UserId,
};

pub async fn get_user_leave_balance(
    Extension(state): Extension<AppState>,
    Extension(actor): Extension<User>,
    Path(user_id): Path<String>,
    Query(query): Query<LeaveBalanceQuery>,
) -> Result<Json<LeaveBalanceResponse>, AppError> {
    let target_user_id =
        UserId::from_str(&user_id).map_err(|_| AppError::BadRequest("Invalid user_id".into()))?;
    authorize_balance_read(&state, &actor, target_user_id).await?;

    let as_of = query.as_of.unwrap_or_else(|| Utc::now().date_naive());
    let repository = LeaveLedgerPostgresRepository::new(state.read_pool().clone());
    let use_case = GetLeaveBalance::new(repository.clone(), repository);
    let view = use_case
        .execute(GetLeaveBalanceCommand { user_id, as_of })
        .await
        .map_err(leave_ledger_error_to_app_error)?;
    Ok(Json(balance_view_to_response(view)))
}

pub async fn run_leave_grants(
    Extension(state): Extension<AppState>,
    Extension(actor): Extension<User>,
    Json(payload): Json<LeaveGrantRunRequest>,
) -> Result<Json<LeaveGrantRunResponse>, AppError> {
    payload.validate()?;
    let repository = LeaveLedgerPostgresRepository::new(state.write_pool.clone());
    let use_case = RunLeaveGrants::new(repository.clone(), repository.clone(), repository);
    let report = use_case
        .execute(RunLeaveGrantsCommand {
            base_date: payload.base_date,
            dry_run: payload.dry_run,
            user_ids: payload.user_ids,
            exclude_user_ids: payload.exclude_user_ids.unwrap_or_default(),
            created_by: Some(actor.id.to_string()),
            now: Utc::now(),
        })
        .await
        .map_err(leave_ledger_error_to_app_error)?;

    Ok(Json(LeaveGrantRunResponse {
        base_date: report.base_date,
        dry_run: report.dry_run,
        granted: report
            .granted
            .into_iter()
            .map(|outcome| LeaveGrantResultResponse {
                user_id: outcome.user_id,
                lot_id: outcome.lot_id,
                tenure_months: outcome.tenure_months,
                granted_minutes: outcome.granted_minutes,
                granted_days: outcome.granted_minutes / outcome.day_equivalent_minutes,
                day_equivalent_minutes: outcome.day_equivalent_minutes,
                granted_at: outcome.granted_at,
                expires_at: outcome.expires_at,
            })
            .collect(),
        skipped: report
            .skipped
            .into_iter()
            .map(|(user_id, reason)| LeaveGrantSkipResponse {
                user_id,
                reason: grant_skip_reason_to_contract(reason),
            })
            .collect(),
        expired: report
            .expired
            .into_iter()
            .map(|expired| LeaveExpiryBackfillResponse {
                user_id: expired.user_id,
                lot_id: expired.lot_id,
                amount_minutes: expired.amount_minutes,
                expires_at: expired.expires_at,
            })
            .collect(),
    }))
}

pub async fn adjust_leave_ledger(
    Extension(state): Extension<AppState>,
    Extension(actor): Extension<User>,
    Json(payload): Json<LeaveLedgerAdjustRequest>,
) -> Result<Json<LeaveLedgerAdjustResponse>, AppError> {
    payload.validate()?;
    let repository = LeaveLedgerPostgresRepository::new(state.write_pool.clone());
    let use_case = AdjustLeaveLedger::new(repository.clone(), repository);
    let report = use_case
        .execute(AdjustLeaveLedgerCommand {
            user_id: payload.user_id,
            amount_minutes: payload.amount_minutes,
            lot_id: payload.lot_id,
            day_equivalent_minutes: payload.day_equivalent_minutes,
            granted_at: payload.granted_at,
            expires_at: payload.expires_at,
            grant_base_date: payload.grant_base_date,
            reason: payload.reason,
            created_by: Some(actor.id.to_string()),
            dry_run: payload.dry_run,
            now: Utc::now(),
        })
        .await
        .map_err(leave_ledger_error_to_app_error)?;

    let balance_after_days = report
        .balance_after
        .active_lots()
        .map(|lot| minutes_to_days(lot.remaining_minutes, lot.day_equivalent_minutes))
        .sum();
    Ok(Json(LeaveLedgerAdjustResponse {
        dry_run: report.dry_run,
        entry: report.entry.map(entry_to_response),
        balance_after_minutes: report.balance_after.available_minutes,
        balance_after_days,
    }))
}

pub async fn set_user_hire_date(
    Extension(state): Extension<AppState>,
    Path(user_id): Path<String>,
    Json(payload): Json<SetHireDateRequest>,
) -> Result<Json<HireDateResponse>, AppError> {
    payload.validate()?;
    let repository = LeaveLedgerPostgresRepository::new(state.write_pool.clone());
    let use_case = SetHireDate::new(repository);
    use_case
        .execute(&user_id, payload.hire_date)
        .await
        .map_err(leave_ledger_error_to_app_error)?;
    Ok(Json(HireDateResponse {
        user_id,
        hire_date: payload.hire_date,
    }))
}

async fn authorize_balance_read(
    state: &AppState,
    actor: &User,
    target_user_id: UserId,
) -> Result<(), AppError> {
    if actor.is_system_admin() {
        return Ok(());
    }
    if actor.is_manager()
        && can_manager_approve(state.read_pool(), actor.id, target_user_id)
            .await
            .map_err(|error| AppError::InternalServerError(error.into()))?
    {
        return Ok(());
    }
    Err(AppError::Forbidden("Forbidden".into()))
}

fn grant_skip_reason_to_contract(reason: GrantSkipReason) -> LeaveGrantSkipReason {
    match reason {
        GrantSkipReason::HireDateNotSet => LeaveGrantSkipReason::HireDateNotSet,
        GrantSkipReason::NotDue => LeaveGrantSkipReason::NotDue,
        GrantSkipReason::AlreadyGranted => LeaveGrantSkipReason::AlreadyGranted,
        GrantSkipReason::NoMatchingRule => LeaveGrantSkipReason::NoMatchingRule,
        GrantSkipReason::Excluded => LeaveGrantSkipReason::Excluded,
    }
}
