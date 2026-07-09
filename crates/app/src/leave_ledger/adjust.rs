//! `AdjustLeaveLedger`: 初期移行・手動調整の adjust 投入（dry-run 照合付き）（T-04）。

use chrono::{DateTime, NaiveDate, Utc};
use timekeeper_domain::leave_ledger::{
    derive_balance, LeaveBalance, LeaveLedgerEvent, LeaveLedgerKind,
};

use super::{
    stored_entries_to_events, LeaveGrantUserRepository, LeaveLedgerError, LeaveLedgerRepository,
    NewLeaveLedgerEntry, StoredLeaveLedgerEntry, ANNUAL_LEAVE_TYPE,
};

// ---------------------------------------------------------------------------
// AdjustLeaveLedger
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdjustLeaveLedgerCommand {
    pub user_id: String,
    pub amount_minutes: i64,
    /// Some: 既存ロットの補正。None: 新規ロット投入（初期移行）。
    pub lot_id: Option<String>,
    pub day_equivalent_minutes: Option<i64>,
    pub granted_at: Option<NaiveDate>,
    pub expires_at: Option<NaiveDate>,
    pub grant_base_date: Option<NaiveDate>,
    pub reason: String,
    pub created_by: Option<String>,
    pub dry_run: bool,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdjustLeaveLedgerReport {
    pub dry_run: bool,
    pub entry: Option<StoredLeaveLedgerEntry>,
    /// 投入（相当）後の導出残高（分、`now` の日付時点）。dry-run 照合に使う。
    pub balance_after: LeaveBalance,
}

#[derive(Debug, Clone)]
pub struct AdjustLeaveLedger<L, U> {
    ledger: L,
    users: U,
}

impl<L, U> AdjustLeaveLedger<L, U>
where
    L: LeaveLedgerRepository,
    U: LeaveGrantUserRepository,
{
    pub fn new(ledger: L, users: U) -> Self {
        Self { ledger, users }
    }

    pub async fn execute(
        &self,
        command: AdjustLeaveLedgerCommand,
    ) -> Result<AdjustLeaveLedgerReport, LeaveLedgerError> {
        if command.reason.trim().is_empty() {
            return Err(LeaveLedgerError::InvalidInput(
                "reason is required for adjust".to_string(),
            ));
        }
        if command.amount_minutes == 0 {
            return Err(LeaveLedgerError::InvalidInput(
                "amount_minutes must not be zero".to_string(),
            ));
        }
        let user_ids = [command.user_id.clone()];
        let candidates = self.users.list_candidates(Some(&user_ids)).await?;
        if candidates.is_empty() {
            return Err(LeaveLedgerError::UserNotFound);
        }

        let as_of = command.now.date_naive();

        if command.dry_run {
            // dry-run は書き込みを行わないため、H-2 のロック対象外
            // (TOCTOU による負残高リスクが無い read-only 経路)。
            let entries = self
                .ledger
                .list_entries(&command.user_id, ANNUAL_LEAVE_TYPE)
                .await?;
            let new_entry = build_adjust_entry(&command, &entries, as_of)?;
            // 仮ロット ID で投入後残高を導出する（新規ロットは未採番のため）。
            let mut hypothetical = stored_entries_to_events(&entries);
            hypothetical.push(new_entry_to_hypothetical_event(&new_entry, as_of));
            return Ok(AdjustLeaveLedgerReport {
                dry_run: true,
                entry: None,
                balance_after: derive_balance(&hypothetical, as_of),
            });
        }

        // H-2: 残高チェック（build_adjust_entry）から追記までを、users 行ロック +
        // 台帳行 FOR UPDATE を取った同一トランザクションで行う（TOCTOU 防止）。
        let command_for_lock = command.clone();
        let (stored, locked_entries) = self
            .ledger
            .with_user_lock(&command.user_id, ANNUAL_LEAVE_TYPE, move |entries| {
                let new_entry = build_adjust_entry(&command_for_lock, entries, as_of)?;
                Ok((vec![new_entry], entries.to_vec()))
            })
            .await?;
        let entry = stored.into_iter().next().ok_or_else(|| {
            LeaveLedgerError::Repository("append_entries returned no entry".to_string())
        })?;
        // 追加のクエリを発行せず、ロック取得時点の entries + 今回追記した entry
        // から投入後残高を導出する（以前は list_entries を再実行していた）。
        let mut events = stored_entries_to_events(&locked_entries);
        events.push(entry.to_domain_event());
        Ok(AdjustLeaveLedgerReport {
            dry_run: false,
            entry: Some(entry),
            balance_after: derive_balance(&events, as_of),
        })
    }
}

/// [`AdjustLeaveLedger::execute`] の dry-run / 実書き込みの両方で使う、
/// 投入する `adjust` entry の構築と検証（残高チェック含む）。
fn build_adjust_entry(
    command: &AdjustLeaveLedgerCommand,
    entries: &[StoredLeaveLedgerEntry],
    as_of: NaiveDate,
) -> Result<NewLeaveLedgerEntry, LeaveLedgerError> {
    match &command.lot_id {
        Some(lot_id) => {
            let events = stored_entries_to_events(entries);
            let balance = derive_balance(&events, as_of);
            let Some(lot) = balance.lots.iter().find(|lot| &lot.lot_id == lot_id) else {
                return Err(LeaveLedgerError::InvalidInput(format!(
                    "unknown lot_id: {lot_id}"
                )));
            };
            // M-2: `+` の代わりに `checked_add` を使う。`amount_minutes` は
            // contract 層（`LeaveLedgerAdjustRequest::amount_minutes`）で
            // i32 範囲にバリデーション済みのはずだが、`dry_run=true` は
            // DB の `i32::try_from` を経由しないため、ここでも防御的に
            // オーバーフローを明示エラーにする（`i64` のラップアラウンドで
            // 負残高チェックを誤通過させない）。
            let new_remaining = lot
                .remaining_minutes
                .checked_add(command.amount_minutes)
                .ok_or_else(|| {
                    LeaveLedgerError::InvalidInput(
                        "amount_minutes overflows the lot balance computation".to_string(),
                    )
                })?;
            if new_remaining < 0 {
                return Err(LeaveLedgerError::InvalidInput(
                    "adjustment would make the lot balance negative".to_string(),
                ));
            }
            Ok(NewLeaveLedgerEntry {
                user_id: command.user_id.clone(),
                leave_type: ANNUAL_LEAVE_TYPE.to_string(),
                kind: LeaveLedgerKind::Adjust,
                lot_id: Some(lot_id.clone()),
                amount_minutes: command.amount_minutes,
                day_equivalent_minutes: lot.day_equivalent_minutes,
                granted_at: None,
                expires_at: None,
                grant_base_date: None,
                leave_request_id: None,
                reason: Some(command.reason.clone()),
                created_by: command.created_by.clone(),
                effective_at: command.now,
            })
        }
        None => {
            if command.amount_minutes <= 0 {
                return Err(LeaveLedgerError::InvalidInput(
                    "a new-lot adjust must have a positive amount".to_string(),
                ));
            }
            let Some(day_equivalent_minutes) = command.day_equivalent_minutes else {
                return Err(LeaveLedgerError::InvalidInput(
                    "day_equivalent_minutes is required for a new lot".to_string(),
                ));
            };
            if day_equivalent_minutes <= 0 {
                return Err(LeaveLedgerError::InvalidInput(
                    "day_equivalent_minutes must be positive".to_string(),
                ));
            }
            let (Some(granted_at), Some(expires_at)) = (command.granted_at, command.expires_at)
            else {
                return Err(LeaveLedgerError::InvalidInput(
                    "granted_at and expires_at are required for a new lot".to_string(),
                ));
            };
            if expires_at <= granted_at {
                return Err(LeaveLedgerError::InvalidInput(
                    "expires_at must be after granted_at".to_string(),
                ));
            }
            Ok(NewLeaveLedgerEntry {
                user_id: command.user_id.clone(),
                leave_type: ANNUAL_LEAVE_TYPE.to_string(),
                kind: LeaveLedgerKind::Adjust,
                lot_id: None,
                amount_minutes: command.amount_minutes,
                day_equivalent_minutes,
                granted_at: Some(granted_at),
                expires_at: Some(expires_at),
                grant_base_date: command.grant_base_date,
                leave_request_id: None,
                reason: Some(command.reason.clone()),
                created_by: command.created_by.clone(),
                effective_at: command.now,
            })
        }
    }
}

/// dry-run 専用: 未採番の新規ロット `NewLeaveLedgerEntry` を、残高導出に使える
/// 仮の `LeaveLedgerEvent` に変換する（実書き込みはしない）。
fn new_entry_to_hypothetical_event(
    entry: &NewLeaveLedgerEntry,
    effective_on: NaiveDate,
) -> LeaveLedgerEvent {
    LeaveLedgerEvent {
        lot_id: entry
            .lot_id
            .clone()
            .unwrap_or_else(|| "(dry-run-new-lot)".to_string()),
        kind: entry.kind,
        amount_minutes: entry.amount_minutes,
        day_equivalent_minutes: entry.day_equivalent_minutes,
        granted_at: entry.granted_at,
        expires_at: entry.expires_at,
        grant_base_date: entry.grant_base_date,
        effective_on,
    }
}
