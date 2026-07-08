//! 有給付与・残高台帳の use case（T-04）。
//!
//! - `GetLeaveBalance`: 残高・時効予定・年5日義務の read-model 導出（保存しない）
//! - `RunLeaveGrants`: 管理者起動の付与実行（dry-run 付き）+ 冪等な expire 補記
//! - `AdjustLeaveLedger`: 初期移行・手動調整の adjust 投入（dry-run 照合付き）
//! - `SetHireDate`: 付与基準日の起点となる入社日の投入
//!
//! 純ロジックは `timekeeper_domain::leave_ledger`、SQL は infra 層に置く。

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use thiserror::Error;
use timekeeper_domain::leave_ledger::{
    allocate_fifo, annual_obligations, derive_balance, grant_expiry_date, pending_expirations,
    scheduled_grant_tenure_months, select_grant_rule, LeaveBalance, LeaveGrantRule,
    LeaveLedgerEvent, LeaveLedgerKind, LeaveObligationRule, ObligationWindow,
};

/// 第一増分で残高連動する休暇種別。
pub const ANNUAL_LEAVE_TYPE: &str = "annual";

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum LeaveLedgerError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("user not found")]
    UserNotFound,
    #[error("leave grant rules are not configured")]
    RulesNotConfigured,
    #[error(
        "insufficient leave balance: requested {requested_minutes}, available {available_minutes}"
    )]
    InsufficientBalance {
        requested_minutes: i64,
        available_minutes: i64,
    },
    #[error("requested leave period contains no working days to consume")]
    NoWorkingDaysInRange,
    #[error("no active leave lot is available to determine day-equivalent minutes")]
    NoActiveLeaveLot,
    #[error("leave ledger repository error: {0}")]
    Repository(String),
}

/// 永続化済み台帳イベント。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredLeaveLedgerEntry {
    pub id: String,
    pub user_id: String,
    pub leave_type: String,
    pub kind: LeaveLedgerKind,
    pub lot_id: String,
    pub amount_minutes: i64,
    pub day_equivalent_minutes: i64,
    pub granted_at: Option<NaiveDate>,
    pub expires_at: Option<NaiveDate>,
    pub grant_base_date: Option<NaiveDate>,
    pub leave_request_id: Option<String>,
    pub reason: Option<String>,
    pub created_by: Option<String>,
    pub effective_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl StoredLeaveLedgerEntry {
    pub fn to_domain_event(&self) -> LeaveLedgerEvent {
        LeaveLedgerEvent {
            lot_id: self.lot_id.clone(),
            kind: self.kind,
            amount_minutes: self.amount_minutes,
            day_equivalent_minutes: self.day_equivalent_minutes,
            granted_at: self.granted_at,
            expires_at: self.expires_at,
            grant_base_date: self.grant_base_date,
            effective_on: self.effective_at.date_naive(),
        }
    }
}

/// 追記する台帳イベント。`lot_id = None` は新規ロット採番を repository へ委ねる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewLeaveLedgerEntry {
    pub user_id: String,
    pub leave_type: String,
    pub kind: LeaveLedgerKind,
    pub lot_id: Option<String>,
    pub amount_minutes: i64,
    pub day_equivalent_minutes: i64,
    pub granted_at: Option<NaiveDate>,
    pub expires_at: Option<NaiveDate>,
    pub grant_base_date: Option<NaiveDate>,
    pub leave_request_id: Option<String>,
    pub reason: Option<String>,
    pub created_by: Option<String>,
    pub effective_at: DateTime<Utc>,
}

#[async_trait]
pub trait LeaveLedgerRepository: Send + Sync {
    async fn list_entries(
        &self,
        user_id: &str,
        leave_type: &str,
    ) -> Result<Vec<StoredLeaveLedgerEntry>, LeaveLedgerError>;

    /// 全 entry を単一トランザクションで追記し、入力順で返す。
    async fn append_entries(
        &self,
        entries: Vec<NewLeaveLedgerEntry>,
    ) -> Result<Vec<StoredLeaveLedgerEntry>, LeaveLedgerError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnualLeaveRequestLedgerCommand {
    pub user_id: String,
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
                leave_type: ANNUAL_LEAVE_TYPE.to_string(),
                kind: LeaveLedgerKind::Consume,
                lot_id: Some(allocation.lot_id),
                amount_minutes: -allocation.amount_minutes,
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
            leave_type: ANNUAL_LEAVE_TYPE.to_string(),
            kind: LeaveLedgerKind::Release,
            lot_id: Some(entry.lot_id.clone()),
            amount_minutes: release_minutes,
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

fn stored_entries_to_events(entries: &[StoredLeaveLedgerEntry]) -> Vec<LeaveLedgerEvent> {
    entries
        .iter()
        .map(StoredLeaveLedgerEntry::to_domain_event)
        .collect()
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

#[async_trait]
pub trait LeaveRuleRepository: Send + Sync {
    /// `on` 時点で有効な付与ルール行（勤続月数ごと）を返す。
    async fn grant_rules(
        &self,
        leave_type: &str,
        on: NaiveDate,
    ) -> Result<Vec<LeaveGrantRule>, LeaveLedgerError>;

    /// `on` 時点で有効な年5日義務ルールを返す（未設定なら None）。
    async fn obligation_rule(
        &self,
        leave_type: &str,
        on: NaiveDate,
    ) -> Result<Option<LeaveObligationRule>, LeaveLedgerError>;
}

/// 付与実行の対象ユーザー。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantCandidate {
    pub user_id: String,
    pub hire_date: Option<NaiveDate>,
}

#[async_trait]
pub trait LeaveGrantUserRepository: Send + Sync {
    /// `user_ids = None` は全ユーザー。指定時は存在するユーザーのみ返す。
    async fn list_candidates(
        &self,
        user_ids: Option<&[String]>,
    ) -> Result<Vec<GrantCandidate>, LeaveLedgerError>;

    async fn set_hire_date(
        &self,
        user_id: &str,
        hire_date: NaiveDate,
    ) -> Result<(), LeaveLedgerError>;
}

// ---------------------------------------------------------------------------
// GetLeaveBalance
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetLeaveBalanceCommand {
    pub user_id: String,
    pub as_of: NaiveDate,
}

/// 残高 read-model（保存されない導出値）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaveBalanceView {
    pub user_id: String,
    pub leave_type: String,
    pub balance: LeaveBalance,
    pub obligations: Vec<ObligationWindow>,
}

#[derive(Debug, Clone)]
pub struct GetLeaveBalance<L, R> {
    ledger: L,
    rules: R,
}

impl<L, R> GetLeaveBalance<L, R>
where
    L: LeaveLedgerRepository,
    R: LeaveRuleRepository,
{
    pub fn new(ledger: L, rules: R) -> Self {
        Self { ledger, rules }
    }

    pub async fn execute(
        &self,
        command: GetLeaveBalanceCommand,
    ) -> Result<LeaveBalanceView, LeaveLedgerError> {
        let entries = self
            .ledger
            .list_entries(&command.user_id, ANNUAL_LEAVE_TYPE)
            .await?;
        let events: Vec<LeaveLedgerEvent> = entries
            .iter()
            .map(|entry| entry.to_domain_event())
            .collect();
        let balance = derive_balance(&events, command.as_of);
        let obligations = match self
            .rules
            .obligation_rule(ANNUAL_LEAVE_TYPE, command.as_of)
            .await?
        {
            Some(rule) => annual_obligations(&events, &rule, command.as_of),
            None => Vec::new(),
        };
        Ok(LeaveBalanceView {
            user_id: command.user_id,
            leave_type: ANNUAL_LEAVE_TYPE.to_string(),
            balance,
            obligations,
        })
    }
}

// ---------------------------------------------------------------------------
// RunLeaveGrants
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunLeaveGrantsCommand {
    pub base_date: NaiveDate,
    pub dry_run: bool,
    /// None は全ユーザー。Some は明示リスト（存在しない ID は UserNotFound）。
    pub user_ids: Option<Vec<String>>,
    pub exclude_user_ids: Vec<String>,
    pub created_by: Option<String>,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantSkipReason {
    HireDateNotSet,
    NotDue,
    AlreadyGranted,
    NoMatchingRule,
    Excluded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantOutcome {
    pub user_id: String,
    /// dry-run では未採番のため None。
    pub lot_id: Option<String>,
    pub tenure_months: i64,
    pub granted_minutes: i64,
    pub day_equivalent_minutes: i64,
    pub granted_at: NaiveDate,
    pub expires_at: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpiryOutcome {
    pub user_id: String,
    pub lot_id: String,
    pub amount_minutes: i64,
    pub expires_at: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunLeaveGrantsReport {
    pub base_date: NaiveDate,
    pub dry_run: bool,
    pub granted: Vec<GrantOutcome>,
    pub skipped: Vec<(String, GrantSkipReason)>,
    pub expired: Vec<ExpiryOutcome>,
}

#[derive(Debug, Clone)]
pub struct RunLeaveGrants<L, R, U> {
    ledger: L,
    rules: R,
    users: U,
}

impl<L, R, U> RunLeaveGrants<L, R, U>
where
    L: LeaveLedgerRepository,
    R: LeaveRuleRepository,
    U: LeaveGrantUserRepository,
{
    pub fn new(ledger: L, rules: R, users: U) -> Self {
        Self {
            ledger,
            rules,
            users,
        }
    }

    pub async fn execute(
        &self,
        command: RunLeaveGrantsCommand,
    ) -> Result<RunLeaveGrantsReport, LeaveLedgerError> {
        if let Some(ids) = &command.user_ids {
            if ids.is_empty() {
                return Err(LeaveLedgerError::InvalidInput(
                    "user_ids must not be empty when provided".to_string(),
                ));
            }
        }
        let rules = self
            .rules
            .grant_rules(ANNUAL_LEAVE_TYPE, command.base_date)
            .await?;
        if rules.is_empty() {
            return Err(LeaveLedgerError::RulesNotConfigured);
        }
        let candidates = self
            .users
            .list_candidates(command.user_ids.as_deref())
            .await?;
        if let Some(ids) = &command.user_ids {
            if candidates.len() != ids.len() {
                return Err(LeaveLedgerError::UserNotFound);
            }
        }

        let mut granted = Vec::new();
        let mut skipped = Vec::new();
        let mut expired = Vec::new();
        let mut new_entries: Vec<NewLeaveLedgerEntry> = Vec::new();
        // append_entries の戻りから grant の lot_id を引くための対応表。
        let mut grant_entry_indexes: Vec<usize> = Vec::new();

        for candidate in candidates {
            if command.exclude_user_ids.contains(&candidate.user_id) {
                skipped.push((candidate.user_id, GrantSkipReason::Excluded));
                continue;
            }
            let entries = self
                .ledger
                .list_entries(&candidate.user_id, ANNUAL_LEAVE_TYPE)
                .await?;
            let events: Vec<LeaveLedgerEvent> = entries
                .iter()
                .map(|entry| entry.to_domain_event())
                .collect();

            // 冪等な expire 補記（付与可否に関わらず全対象者に対して行う）。
            let balance = derive_balance(&events, command.base_date);
            for draft in pending_expirations(&balance) {
                expired.push(ExpiryOutcome {
                    user_id: candidate.user_id.clone(),
                    lot_id: draft.lot_id.clone(),
                    amount_minutes: draft.amount_minutes,
                    expires_at: draft.expires_at,
                });
                new_entries.push(NewLeaveLedgerEntry {
                    user_id: candidate.user_id.clone(),
                    leave_type: ANNUAL_LEAVE_TYPE.to_string(),
                    kind: LeaveLedgerKind::Expire,
                    lot_id: Some(draft.lot_id),
                    amount_minutes: draft.amount_minutes,
                    day_equivalent_minutes: draft.day_equivalent_minutes,
                    granted_at: None,
                    expires_at: None,
                    grant_base_date: None,
                    leave_request_id: None,
                    reason: None,
                    created_by: command.created_by.clone(),
                    effective_at: start_of_day_utc(draft.expires_at),
                });
            }

            let Some(hire_date) = candidate.hire_date else {
                skipped.push((candidate.user_id, GrantSkipReason::HireDateNotSet));
                continue;
            };
            let Some(tenure_months) = scheduled_grant_tenure_months(hire_date, command.base_date)
            else {
                skipped.push((candidate.user_id, GrantSkipReason::NotDue));
                continue;
            };
            let already_granted = entries.iter().any(|entry| {
                entry.kind == LeaveLedgerKind::Grant
                    && entry.grant_base_date == Some(command.base_date)
            });
            if already_granted {
                skipped.push((candidate.user_id, GrantSkipReason::AlreadyGranted));
                continue;
            }
            let Some(rule) = select_grant_rule(&rules, tenure_months) else {
                skipped.push((candidate.user_id, GrantSkipReason::NoMatchingRule));
                continue;
            };
            let Some(expires_at) = grant_expiry_date(command.base_date, rule.expiry_months) else {
                return Err(LeaveLedgerError::InvalidInput(format!(
                    "cannot compute expiry date for base date {}",
                    command.base_date
                )));
            };
            let granted_minutes = rule.granted_days * rule.day_equivalent_minutes;
            granted.push(GrantOutcome {
                user_id: candidate.user_id.clone(),
                lot_id: None,
                tenure_months,
                granted_minutes,
                day_equivalent_minutes: rule.day_equivalent_minutes,
                granted_at: command.base_date,
                expires_at,
            });
            grant_entry_indexes.push(new_entries.len());
            new_entries.push(NewLeaveLedgerEntry {
                user_id: candidate.user_id,
                leave_type: ANNUAL_LEAVE_TYPE.to_string(),
                kind: LeaveLedgerKind::Grant,
                lot_id: None,
                amount_minutes: granted_minutes,
                day_equivalent_minutes: rule.day_equivalent_minutes,
                granted_at: Some(command.base_date),
                expires_at: Some(expires_at),
                grant_base_date: Some(command.base_date),
                leave_request_id: None,
                reason: None,
                created_by: command.created_by.clone(),
                effective_at: start_of_day_utc(command.base_date),
            });
        }

        if !command.dry_run && !new_entries.is_empty() {
            let stored = self.ledger.append_entries(new_entries).await?;
            for (outcome, entry_index) in granted.iter_mut().zip(grant_entry_indexes) {
                if let Some(entry) = stored.get(entry_index) {
                    outcome.lot_id = Some(entry.lot_id.clone());
                }
            }
        }

        Ok(RunLeaveGrantsReport {
            base_date: command.base_date,
            dry_run: command.dry_run,
            granted,
            skipped,
            expired,
        })
    }
}

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

        let entries = self
            .ledger
            .list_entries(&command.user_id, ANNUAL_LEAVE_TYPE)
            .await?;
        let events: Vec<LeaveLedgerEvent> = entries
            .iter()
            .map(|entry| entry.to_domain_event())
            .collect();
        let as_of = command.now.date_naive();

        let new_entry = match &command.lot_id {
            Some(lot_id) => {
                let balance = derive_balance(&events, as_of);
                let Some(lot) = balance.lots.iter().find(|lot| &lot.lot_id == lot_id) else {
                    return Err(LeaveLedgerError::InvalidInput(format!(
                        "unknown lot_id: {lot_id}"
                    )));
                };
                if lot.remaining_minutes + command.amount_minutes < 0 {
                    return Err(LeaveLedgerError::InvalidInput(
                        "adjustment would make the lot balance negative".to_string(),
                    ));
                }
                NewLeaveLedgerEntry {
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
                }
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
                NewLeaveLedgerEntry {
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
                }
            }
        };

        if command.dry_run {
            // 仮ロット ID で投入後残高を導出する（新規ロットは未採番のため）。
            let mut hypothetical = events;
            hypothetical.push(LeaveLedgerEvent {
                lot_id: new_entry
                    .lot_id
                    .clone()
                    .unwrap_or_else(|| "(dry-run-new-lot)".to_string()),
                kind: LeaveLedgerKind::Adjust,
                amount_minutes: new_entry.amount_minutes,
                day_equivalent_minutes: new_entry.day_equivalent_minutes,
                granted_at: new_entry.granted_at,
                expires_at: new_entry.expires_at,
                grant_base_date: new_entry.grant_base_date,
                effective_on: as_of,
            });
            return Ok(AdjustLeaveLedgerReport {
                dry_run: true,
                entry: None,
                balance_after: derive_balance(&hypothetical, as_of),
            });
        }

        let stored = self.ledger.append_entries(vec![new_entry]).await?;
        let entry = stored.into_iter().next().ok_or_else(|| {
            LeaveLedgerError::Repository("append_entries returned no entry".to_string())
        })?;
        let entries = self
            .ledger
            .list_entries(&command.user_id, ANNUAL_LEAVE_TYPE)
            .await?;
        let events: Vec<LeaveLedgerEvent> = entries
            .iter()
            .map(|entry| entry.to_domain_event())
            .collect();
        Ok(AdjustLeaveLedgerReport {
            dry_run: false,
            entry: Some(entry),
            balance_after: derive_balance(&events, as_of),
        })
    }
}

// ---------------------------------------------------------------------------
// SetHireDate
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SetHireDate<U> {
    users: U,
}

impl<U> SetHireDate<U>
where
    U: LeaveGrantUserRepository,
{
    pub fn new(users: U) -> Self {
        Self { users }
    }

    pub async fn execute(
        &self,
        user_id: &str,
        hire_date: NaiveDate,
    ) -> Result<(), LeaveLedgerError> {
        if user_id.trim().is_empty() {
            return Err(LeaveLedgerError::InvalidInput(
                "user_id is required".to_string(),
            ));
        }
        self.users.set_hire_date(user_id, hire_date).await
    }
}

fn start_of_day_utc(date: NaiveDate) -> DateTime<Utc> {
    DateTime::<Utc>::from_naive_utc_and_offset(date.and_hms_opt(0, 0, 0).expect("midnight"), Utc)
}
