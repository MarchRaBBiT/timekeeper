//! M-6: `LeaveLedgerError` → `AppError` の変換。
//!
//! この変換は元々 `backend/src/handlers/leave_ledger.rs` と
//! `backend/src/repositories/request.rs` の 2 箇所に同一実装がコピーされていた
//! （handler / repository の層違反を避けつつ、両方から使える中立の場所として
//! `crate::error` 配下に 1 本化する）。

use timekeeper_app::leave_ledger::LeaveLedgerError;
use timekeeper_contract::leave::{
    LEAVE_BALANCE_INSUFFICIENT_CODE, LEAVE_REQUEST_NO_ACTIVE_LOT_CODE,
    LEAVE_REQUEST_NO_WORKING_DAYS_CODE,
};

use crate::error::AppError;

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
        LeaveLedgerError::BatchAlreadyRunning => {
            AppError::Conflict("A leave grant batch is already running; please retry later".into())
        }
        LeaveLedgerError::Repository(message) => {
            AppError::InternalServerError(anyhow::anyhow!(message))
        }
    }
}
