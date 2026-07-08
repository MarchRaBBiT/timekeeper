use axum::{extract::Query, Extension, Json};
use timekeeper_app::leave_ledger::{
    GetLeaveBalance, GetLeaveBalanceCommand, LeaveBalanceView, LeaveLedgerError,
    StoredLeaveLedgerEntry,
};
use timekeeper_contract::leave::{
    LeaveBalanceQuery, LeaveBalanceResponse, LeaveExpiryScheduleResponse, LeaveLedgerEntryResponse,
    LeaveLedgerKind as ContractLeaveLedgerKind, LeaveLotResponse, LeaveObligationStatus,
    LeaveObligationWindowResponse, LEAVE_BALANCE_INSUFFICIENT_CODE,
    LEAVE_REQUEST_NO_ACTIVE_LOT_CODE, LEAVE_REQUEST_NO_WORKING_DAYS_CODE,
};
use timekeeper_domain::leave_ledger::{LeaveLedgerKind, ObligationStatus};
use timekeeper_infra_postgres::leave_ledger::LeaveLedgerPostgresRepository;

use crate::{error::AppError, models::user::User, state::AppState};

pub async fn get_my_leave_balance(
    Extension(state): Extension<AppState>,
    Extension(user): Extension<User>,
    Query(query): Query<LeaveBalanceQuery>,
) -> Result<Json<LeaveBalanceResponse>, AppError> {
    let as_of = query
        .as_of
        .unwrap_or_else(|| chrono::Utc::now().date_naive());
    let repository = LeaveLedgerPostgresRepository::new(state.read_pool().clone());
    let use_case = GetLeaveBalance::new(repository.clone(), repository);
    let view = use_case
        .execute(GetLeaveBalanceCommand {
            user_id: user.id.to_string(),
            as_of,
        })
        .await
        .map_err(leave_ledger_error_to_app_error)?;
    Ok(Json(balance_view_to_response(view)))
}

pub fn balance_view_to_response(view: LeaveBalanceView) -> LeaveBalanceResponse {
    let active_lots: Vec<LeaveLotResponse> = view
        .balance
        .active_lots()
        .map(|lot| LeaveLotResponse {
            lot_id: lot.lot_id.clone(),
            granted_at: lot.granted_at,
            expires_at: lot.expires_at,
            grant_base_date: lot.grant_base_date,
            day_equivalent_minutes: lot.day_equivalent_minutes,
            granted_minutes: lot.granted_minutes,
            remaining_minutes: lot.remaining_minutes,
        })
        .collect();
    let available_days = active_lots
        .iter()
        .map(|lot| minutes_to_days(lot.remaining_minutes, lot.day_equivalent_minutes))
        .sum();
    let upcoming_expiries = active_lots
        .iter()
        .filter_map(|lot| {
            lot.expires_at
                .map(|expires_at| LeaveExpiryScheduleResponse {
                    lot_id: lot.lot_id.clone(),
                    expires_at,
                    remaining_minutes: lot.remaining_minutes,
                })
        })
        .collect();
    let obligations = view
        .obligations
        .into_iter()
        .map(|window| LeaveObligationWindowResponse {
            grant_base_date: window.grant_base_date,
            window_end: window.window_end,
            day_equivalent_minutes: window.day_equivalent_minutes,
            granted_minutes: window.granted_minutes,
            required_minutes: window.required_minutes,
            taken_minutes: window.taken_minutes,
            required_days: minutes_to_days(window.required_minutes, window.day_equivalent_minutes),
            taken_days: minutes_to_days(window.taken_minutes, window.day_equivalent_minutes),
            status: obligation_status_to_contract(window.status),
        })
        .collect();

    LeaveBalanceResponse {
        user_id: view.user_id,
        leave_type: view.leave_type,
        as_of: view.balance.as_of,
        available_minutes: view.balance.available_minutes,
        available_days,
        active_lots,
        upcoming_expiries,
        obligations,
    }
}

pub fn entry_to_response(entry: StoredLeaveLedgerEntry) -> LeaveLedgerEntryResponse {
    LeaveLedgerEntryResponse {
        id: entry.id,
        user_id: entry.user_id,
        leave_type: entry.leave_type,
        kind: ledger_kind_to_contract(entry.kind),
        lot_id: entry.lot_id,
        amount_minutes: entry.amount_minutes,
        day_equivalent_minutes: entry.day_equivalent_minutes,
        granted_at: entry.granted_at,
        expires_at: entry.expires_at,
        grant_base_date: entry.grant_base_date,
        leave_request_id: entry.leave_request_id,
        reason: entry.reason,
        created_by: entry.created_by,
        effective_at: entry.effective_at,
        created_at: entry.created_at,
    }
}

pub fn leave_ledger_error_to_app_error(error: LeaveLedgerError) -> AppError {
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
        LeaveLedgerError::Repository(message) => {
            AppError::InternalServerError(anyhow::anyhow!(message))
        }
    }
}

pub fn minutes_to_days(minutes: i64, day_equivalent_minutes: i64) -> f64 {
    if day_equivalent_minutes <= 0 {
        return 0.0;
    }
    minutes as f64 / day_equivalent_minutes as f64
}

pub fn ledger_kind_to_contract(kind: LeaveLedgerKind) -> ContractLeaveLedgerKind {
    match kind {
        LeaveLedgerKind::Grant => ContractLeaveLedgerKind::Grant,
        LeaveLedgerKind::Consume => ContractLeaveLedgerKind::Consume,
        LeaveLedgerKind::Release => ContractLeaveLedgerKind::Release,
        LeaveLedgerKind::Expire => ContractLeaveLedgerKind::Expire,
        LeaveLedgerKind::Adjust => ContractLeaveLedgerKind::Adjust,
    }
}

fn obligation_status_to_contract(status: ObligationStatus) -> LeaveObligationStatus {
    match status {
        ObligationStatus::Ok => LeaveObligationStatus::Ok,
        ObligationStatus::Warning => LeaveObligationStatus::Warning,
        ObligationStatus::AtRisk => LeaveObligationStatus::AtRisk,
        ObligationStatus::Fulfilled => LeaveObligationStatus::Fulfilled,
    }
}
