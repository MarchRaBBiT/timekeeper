//! 有給申請の消化（consume）・取消時解放（release）entries 構築（T-04 / H-1）。

use chrono::NaiveDate;
use timekeeper_domain::leave_ledger::{
    allocate_fifo, derive_balance, LeaveBalance, LeaveLedgerKind,
};

use super::{
    start_of_day_utc, stored_entries_to_events, LeaveLedgerError, NewLeaveLedgerEntry,
    StoredLeaveLedgerEntry,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnualLeaveRequestLedgerCommand {
    pub user_id: String,
    pub leave_type_code: String,
    pub request_id: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub created_by: Option<String>,
}

/// `workday_count` は申請期間中で resolved workday が稼働日（`ScheduledWorkday`）と
/// 判定した日数（暦日数ではない）。呼び出し元（backend repository 層）が resolved
/// workday を解決して渡す。勤務予定を解決できない日を含む場合は、呼び出し元が
/// fail-closed でこの関数を呼ばずにエラーを返すこと（docs/design-docs/leave-entitlement.md
/// の Consumption Target Days 決定）。
pub fn ensure_annual_leave_request_has_balance(
    entries: &[StoredLeaveLedgerEntry],
    start_date: NaiveDate,
    end_date: NaiveDate,
    workday_count: i64,
) -> Result<(), LeaveLedgerError> {
    if start_date > end_date {
        return Err(LeaveLedgerError::InvalidInput(
            "start_date must be <= end_date".to_string(),
        ));
    }
    let events = stored_entries_to_events(entries);
    let balance = derive_balance(&events, start_date);
    let requested_minutes = requested_minutes_for_workdays(&balance, workday_count)?;
    allocate_fifo(&balance, requested_minutes)
        .map(|_| ())
        .map_err(allocation_error_to_ledger_error)
}

/// 分数が既に勤務予定から正規化されている部分休暇の残高を検証する。
pub fn ensure_leave_request_minutes_have_balance(
    entries: &[StoredLeaveLedgerEntry],
    as_of: NaiveDate,
    requested_minutes: i64,
) -> Result<(), LeaveLedgerError> {
    if requested_minutes <= 0 {
        return Err(LeaveLedgerError::InvalidInput(
            "leave request duration must be positive".to_string(),
        ));
    }
    let balance = derive_balance(&stored_entries_to_events(entries), as_of);
    allocate_fifo(&balance, requested_minutes)
        .map(|_| ())
        .map_err(allocation_error_to_ledger_error)
}

/// `workday_count` の意味は [`ensure_annual_leave_request_has_balance`] と同じ。
pub fn build_annual_leave_consume_entries(
    entries: &[StoredLeaveLedgerEntry],
    command: AnnualLeaveRequestLedgerCommand,
    workday_count: i64,
) -> Result<Vec<NewLeaveLedgerEntry>, LeaveLedgerError> {
    if command.start_date > command.end_date {
        return Err(LeaveLedgerError::InvalidInput(
            "start_date must be <= end_date".to_string(),
        ));
    }
    let events = stored_entries_to_events(entries);
    let balance = derive_balance(&events, command.start_date);
    let requested_minutes = requested_minutes_for_workdays(&balance, workday_count)?;
    let allocations =
        allocate_fifo(&balance, requested_minutes).map_err(allocation_error_to_ledger_error)?;
    allocations
        .into_iter()
        .map(|allocation| {
            let lot = balance
                .lots
                .iter()
                .find(|lot| lot.lot_id == allocation.lot_id)
                .ok_or_else(|| {
                    LeaveLedgerError::Repository(format!(
                        "allocated lot was not found: {}",
                        allocation.lot_id
                    ))
                })?;
            Ok(NewLeaveLedgerEntry {
                user_id: command.user_id.clone(),
                leave_type: command.leave_type_code.clone(),
                kind: LeaveLedgerKind::Consume,
                lot_id: Some(allocation.lot_id),
                amount_minutes: -allocation.amount_minutes,
                obligation_minutes: -allocation.amount_minutes,
                day_equivalent_minutes: lot.day_equivalent_minutes,
                granted_at: None,
                expires_at: None,
                grant_base_date: None,
                leave_request_id: Some(command.request_id.clone()),
                reason: None,
                created_by: command.created_by.clone(),
                effective_at: start_of_day_utc(command.start_date),
            })
        })
        .collect()
}

pub fn build_leave_consume_entries_for_minutes(
    entries: &[StoredLeaveLedgerEntry],
    command: AnnualLeaveRequestLedgerCommand,
    requested_minutes: i64,
    obligation_half_day: bool,
) -> Result<Vec<NewLeaveLedgerEntry>, LeaveLedgerError> {
    if requested_minutes <= 0 {
        return Err(LeaveLedgerError::InvalidInput(
            "leave request duration must be positive".to_string(),
        ));
    }
    let balance = derive_balance(&stored_entries_to_events(entries), command.start_date);
    let allocations =
        allocate_fifo(&balance, requested_minutes).map_err(allocation_error_to_ledger_error)?;
    let mut obligation_recorded = false;
    allocations
        .into_iter()
        .map(|allocation| {
            let lot = balance
                .lots
                .iter()
                .find(|lot| lot.lot_id == allocation.lot_id)
                .ok_or_else(|| {
                    LeaveLedgerError::Repository(format!(
                        "allocated lot was not found: {}",
                        allocation.lot_id
                    ))
                })?;
            Ok(NewLeaveLedgerEntry {
                user_id: command.user_id.clone(),
                leave_type: command.leave_type_code.clone(),
                kind: LeaveLedgerKind::Consume,
                lot_id: Some(allocation.lot_id),
                amount_minutes: -allocation.amount_minutes,
                obligation_minutes: if obligation_half_day && !obligation_recorded {
                    obligation_recorded = true;
                    -(lot.day_equivalent_minutes / 2)
                } else {
                    0
                },
                day_equivalent_minutes: lot.day_equivalent_minutes,
                granted_at: None,
                expires_at: None,
                grant_base_date: None,
                leave_request_id: Some(command.request_id.clone()),
                reason: None,
                created_by: command.created_by.clone(),
                effective_at: start_of_day_utc(command.start_date),
            })
        })
        .collect()
}

pub fn build_annual_leave_release_entries(
    entries: &[StoredLeaveLedgerEntry],
    command: AnnualLeaveRequestLedgerCommand,
) -> Vec<NewLeaveLedgerEntry> {
    let mut releases = Vec::new();
    for entry in entries.iter().filter(|entry| {
        entry.leave_request_id.as_deref() == Some(command.request_id.as_str())
            && entry.kind == LeaveLedgerKind::Consume
    }) {
        let already_released: i64 = entries
            .iter()
            .filter(|candidate| {
                candidate.leave_request_id.as_deref() == Some(command.request_id.as_str())
                    && candidate.kind == LeaveLedgerKind::Release
                    && candidate.lot_id == entry.lot_id
            })
            .map(|candidate| candidate.amount_minutes)
            .sum();
        let consumed_minutes = -entry.amount_minutes;
        let release_minutes = consumed_minutes - already_released;
        if release_minutes <= 0 {
            continue;
        }
        releases.push(NewLeaveLedgerEntry {
            user_id: command.user_id.clone(),
            leave_type: command.leave_type_code.clone(),
            kind: LeaveLedgerKind::Release,
            lot_id: Some(entry.lot_id.clone()),
            amount_minutes: release_minutes,
            obligation_minutes: -entry.obligation_minutes,
            day_equivalent_minutes: entry.day_equivalent_minutes,
            granted_at: None,
            expires_at: None,
            grant_base_date: None,
            leave_request_id: Some(command.request_id.clone()),
            reason: None,
            created_by: command.created_by.clone(),
            effective_at: start_of_day_utc(command.start_date),
        });
    }
    releases
}

fn allocation_error_to_ledger_error(
    error: timekeeper_domain::leave_ledger::FifoAllocationError,
) -> LeaveLedgerError {
    match error {
        timekeeper_domain::leave_ledger::FifoAllocationError::NonPositiveAmount => {
            LeaveLedgerError::InvalidInput("leave request duration must be positive".to_string())
        }
        timekeeper_domain::leave_ledger::FifoAllocationError::InsufficientBalance {
            requested_minutes,
            available_minutes,
        } => LeaveLedgerError::InsufficientBalance {
            requested_minutes,
            available_minutes,
        },
    }
}

/// 稼働日数（暦日数ではない）を、アクティブなロットの `day_equivalent_minutes` で
/// 分へ換算する。稼働日が 0 日の申請、およびアクティブなロットが存在せず換算基準が
/// 無い場合は、いずれも明示的なエラーとして拒否する（暦日フォールバック・固定値
/// フォールバックはしない）。
fn requested_minutes_for_workdays(
    balance: &LeaveBalance,
    workday_count: i64,
) -> Result<i64, LeaveLedgerError> {
    if workday_count <= 0 {
        return Err(LeaveLedgerError::NoWorkingDaysInRange);
    }
    let day_equivalent_minutes = balance
        .active_lots()
        .next()
        .map(|lot| lot.day_equivalent_minutes)
        .ok_or(LeaveLedgerError::NoActiveLeaveLot)?;
    Ok(workday_count * day_equivalent_minutes)
}
