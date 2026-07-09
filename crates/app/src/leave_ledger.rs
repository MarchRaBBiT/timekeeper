//! 有給付与・残高台帳の use case（T-04）。
//!
//! - `GetLeaveBalance`: 残高・時効予定・年5日義務の read-model 導出（保存しない）
//! - `RunLeaveGrants`: 管理者起動の付与実行（dry-run 付き）+ 冪等な expire 補記
//! - `AdjustLeaveLedger`: 初期移行・手動調整の adjust 投入（dry-run 照合付き）
//! - `SetHireDate`: 付与基準日の起点となる入社日の投入
//!
//! 純ロジックは `timekeeper_domain::leave_ledger`、SQL は infra 層に置く。

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

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
    /// M-1a: 付与バッチの二重起動防止。既に実行中のバッチがある間は
    /// 新しいバッチ実行をブロック待ちさせず、即座にこのエラーで拒否する。
    #[error("a leave grant batch is already running")]
    BatchAlreadyRunning,
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

/// [`LeaveLedgerRepository::with_user_lock`] が返す future の型。
///
/// トレイトメソッドが `user_id` 単位のロックとジェネリックな `compute` を
/// 両立するため、`async_trait` ではなく手書きの `Pin<Box<dyn Future>>` で
/// 表現する（`async_trait` はメソッドジェネリクスと相性が悪いため）。
pub type LedgerLockFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, LeaveLedgerError>> + Send + 'a>>;

#[async_trait]
pub trait LeaveLedgerRepository: Send + Sync {
    async fn list_entries(
        &self,
        user_id: &str,
        leave_type: &str,
    ) -> Result<Vec<StoredLeaveLedgerEntry>, LeaveLedgerError>;

    /// 複数ユーザー分の entries をロックなしで一括取得する。
    ///
    /// N+1 SELECT を避けるためのバッチ処理の事前絞り込み専用（H-2 / M-1b）。
    /// この結果を根拠に書き込み判定を確定してはならない。実際に書き込みを
    /// 行う判定は、必ず [`LeaveLedgerRepository::with_user_lock`] で取得した
    /// ロック済み entries で再検証すること。
    async fn list_entries_for_users(
        &self,
        user_ids: &[String],
        leave_type: &str,
    ) -> Result<Vec<StoredLeaveLedgerEntry>, LeaveLedgerError>;

    /// `user_id` の `users` 行をロックしたうえで台帳 entries を FOR UPDATE 取得し、
    /// `compute` が返す entries を同一トランザクションで追記する（H-2 の
    /// TOCTOU 修正）。`compute` が返す entries が空なら追記をスキップする
    /// （純粋な read-only 判定にも使える）。
    ///
    /// ロック順序は `users` → `leave_ledger_entries` に統一し、
    /// `backend/src/repositories/request.rs` の承認・取消経路（既存の
    /// `lock_user_for_ledger` → `list_entries_for_update`）と揃える
    /// （デッドロック防止）。
    fn with_user_lock<'a, F, T>(
        &'a self,
        user_id: &'a str,
        leave_type: &'a str,
        compute: F,
    ) -> LedgerLockFuture<'a, (Vec<StoredLeaveLedgerEntry>, T)>
    where
        F: FnOnce(
                &[StoredLeaveLedgerEntry],
            ) -> Result<(Vec<NewLeaveLedgerEntry>, T), LeaveLedgerError>
            + Send
            + 'a,
        T: Send + 'a;
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

/// ユーザー単位トランザクションの書き込みが失敗した記録（M-1a）。
///
/// 既存の `LeaveGrantRunResponse` API 契約は変更しないため、この情報は
/// レスポンス JSON へはマッピングしない。呼び出し元（handler）で
/// `tracing` 等へログ出力し、運用上のフォローに使う。失敗したユーザーは
/// 冪等な再実行（同じ `base_date` での再実行）で再度対象になる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantFailure {
    pub user_id: String,
    pub error: LeaveLedgerError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunLeaveGrantsReport {
    pub base_date: NaiveDate,
    pub dry_run: bool,
    pub granted: Vec<GrantOutcome>,
    pub skipped: Vec<(String, GrantSkipReason)>,
    pub expired: Vec<ExpiryOutcome>,
    pub failed: Vec<GrantFailure>,
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

        let mut acc = GrantRunAccumulator::default();
        let mut active_candidates = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            if command.exclude_user_ids.contains(&candidate.user_id) {
                acc.skipped
                    .push((candidate.user_id, GrantSkipReason::Excluded));
            } else {
                active_candidates.push(candidate);
            }
        }

        // M-1b: 候補者ごとの N+1 SELECT を避けるため、ロックなしで一括取得する。
        // これは事前絞り込み専用。実際に書き込むかどうかの判定は、必ず
        // with_user_lock で取得したロック済み entries で再検証する。
        let entries_by_user = self.bulk_entries_by_user(&active_candidates).await?;

        if command.dry_run {
            collect_dry_run(
                &active_candidates,
                &entries_by_user,
                &rules,
                &command,
                &mut acc,
            )?;
        } else {
            self.collect_with_locks(
                active_candidates,
                &entries_by_user,
                &rules,
                &command,
                &mut acc,
            )
            .await?;
        }

        Ok(RunLeaveGrantsReport {
            base_date: command.base_date,
            dry_run: command.dry_run,
            granted: acc.granted,
            skipped: acc.skipped,
            expired: acc.expired,
            failed: acc.failed,
        })
    }

    async fn bulk_entries_by_user(
        &self,
        candidates: &[GrantCandidate],
    ) -> Result<HashMap<String, Vec<StoredLeaveLedgerEntry>>, LeaveLedgerError> {
        let user_ids: Vec<String> = candidates
            .iter()
            .map(|candidate| candidate.user_id.clone())
            .collect();
        let bulk_entries = self
            .ledger
            .list_entries_for_users(&user_ids, ANNUAL_LEAVE_TYPE)
            .await?;
        let mut entries_by_user: HashMap<String, Vec<StoredLeaveLedgerEntry>> = HashMap::new();
        for entry in bulk_entries {
            entries_by_user
                .entry(entry.user_id.clone())
                .or_default()
                .push(entry);
        }
        Ok(entries_by_user)
    }

    /// H-2: 各ユーザーの読み取り〜書き込みを users 行ロック + 台帳行
    /// FOR UPDATE を取った単一トランザクションに閉じる（TOCTOU 防止）。
    /// M-1a: ユーザー単位でトランザクションを分離しているため、1 人の
    /// 失敗（unique 制約違反など）が他ユーザーの付与・expire 補記を
    /// ロールバックしない。
    async fn collect_with_locks(
        &self,
        candidates: Vec<GrantCandidate>,
        entries_by_user: &HashMap<String, Vec<StoredLeaveLedgerEntry>>,
        rules: &[LeaveGrantRule],
        command: &RunLeaveGrantsCommand,
        acc: &mut GrantRunAccumulator,
    ) -> Result<(), LeaveLedgerError> {
        for candidate in candidates {
            let entries = entries_by_user
                .get(&candidate.user_id)
                .map(Vec::as_slice)
                .unwrap_or_default();
            let (preview, preview_entries) = evaluate_candidate(
                &candidate,
                entries,
                rules,
                command.base_date,
                &command.created_by,
            )?;
            if preview_entries.is_empty() {
                if let Some(reason) = preview.skip {
                    acc.skipped.push((candidate.user_id, reason));
                }
                continue;
            }

            match self.grant_with_lock(&candidate, rules, command).await {
                Ok((stored, outcome)) => {
                    acc.record(candidate.user_id, outcome, &stored);
                }
                // 系統的な設定不備（付与ルールの時効月数から日付を算出できない
                // 等）は全ユーザーに影響するため、バッチ全体を中断する。
                Err(LeaveLedgerError::InvalidInput(message)) => {
                    return Err(LeaveLedgerError::InvalidInput(message));
                }
                // それ以外（ロック/DB 起因のエラー）は当該ユーザーのみ
                // failed として記録し、他ユーザーの処理は継続する。
                Err(error) => {
                    acc.failed.push(GrantFailure {
                        user_id: candidate.user_id,
                        error,
                    });
                }
            }
        }
        Ok(())
    }

    /// ロック済み entries で判定し直してから同一トランザクションで追記する。
    async fn grant_with_lock(
        &self,
        candidate: &GrantCandidate,
        rules: &[LeaveGrantRule],
        command: &RunLeaveGrantsCommand,
    ) -> Result<(Vec<StoredLeaveLedgerEntry>, CandidateOutcome), LeaveLedgerError> {
        let candidate_for_lock = candidate.clone();
        let rules_for_lock = rules.to_vec();
        let created_by_for_lock = command.created_by.clone();
        let base_date = command.base_date;
        self.ledger
            .with_user_lock(
                &candidate.user_id,
                ANNUAL_LEAVE_TYPE,
                move |locked_entries| {
                    let (outcome, new_entries) = evaluate_candidate(
                        &candidate_for_lock,
                        locked_entries,
                        &rules_for_lock,
                        base_date,
                        &created_by_for_lock,
                    )?;
                    Ok((new_entries, outcome))
                },
            )
            .await
    }
}

/// dry-run: ロックなしの一括読み取りだけで結果を導出する（書き込みなし）。
fn collect_dry_run(
    candidates: &[GrantCandidate],
    entries_by_user: &HashMap<String, Vec<StoredLeaveLedgerEntry>>,
    rules: &[LeaveGrantRule],
    command: &RunLeaveGrantsCommand,
    acc: &mut GrantRunAccumulator,
) -> Result<(), LeaveLedgerError> {
    for candidate in candidates {
        let entries = entries_by_user
            .get(&candidate.user_id)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let (outcome, _new_entries) = evaluate_candidate(
            candidate,
            entries,
            rules,
            command.base_date,
            &command.created_by,
        )?;
        acc.record(candidate.user_id.clone(), outcome, &[]);
    }
    Ok(())
}

/// `RunLeaveGrants::execute` の集計バッファ。
#[derive(Default)]
struct GrantRunAccumulator {
    granted: Vec<GrantOutcome>,
    skipped: Vec<(String, GrantSkipReason)>,
    expired: Vec<ExpiryOutcome>,
    failed: Vec<GrantFailure>,
}

impl GrantRunAccumulator {
    /// 1 候補者分の判定結果を集計へ反映する。`stored` が非空なら末尾の entry
    /// （grant は必ず最後に追記される）から採番済み lot_id を補完する。
    fn record(
        &mut self,
        user_id: String,
        outcome: CandidateOutcome,
        stored: &[StoredLeaveLedgerEntry],
    ) {
        self.expired.extend(outcome.expired);
        match outcome.grant {
            Some(mut grant) => {
                if let Some(entry) = stored.last() {
                    grant.lot_id = Some(entry.lot_id.clone());
                }
                self.granted.push(grant);
            }
            None => {
                if let Some(reason) = outcome.skip {
                    self.skipped.push((user_id, reason));
                }
            }
        }
    }
}

/// 1 候補者分の付与可否と冪等 expire 補記を判定する（純ロジック）。
///
/// `entries` は呼び出し元がロック付き／ロックなしのどちらで取得したかを
/// 問わない。書き込みを伴う判定にこの関数を使う場合、呼び出し元が
/// `with_user_lock` 等でトランザクション境界を保証すること。
fn evaluate_candidate(
    candidate: &GrantCandidate,
    entries: &[StoredLeaveLedgerEntry],
    rules: &[LeaveGrantRule],
    base_date: NaiveDate,
    created_by: &Option<String>,
) -> Result<(CandidateOutcome, Vec<NewLeaveLedgerEntry>), LeaveLedgerError> {
    let events = stored_entries_to_events(entries);
    let balance = derive_balance(&events, base_date);
    let (expired, mut new_entries) = expiration_entries(candidate, &balance, created_by);

    let Some(hire_date) = candidate.hire_date else {
        return Ok(skip_outcome(
            expired,
            new_entries,
            GrantSkipReason::HireDateNotSet,
        ));
    };
    let Some(tenure_months) = scheduled_grant_tenure_months(hire_date, base_date) else {
        return Ok(skip_outcome(expired, new_entries, GrantSkipReason::NotDue));
    };
    let already_granted = entries.iter().any(|entry| {
        entry.kind == LeaveLedgerKind::Grant && entry.grant_base_date == Some(base_date)
    });
    if already_granted {
        return Ok(skip_outcome(
            expired,
            new_entries,
            GrantSkipReason::AlreadyGranted,
        ));
    }
    let Some(rule) = select_grant_rule(rules, tenure_months) else {
        return Ok(skip_outcome(
            expired,
            new_entries,
            GrantSkipReason::NoMatchingRule,
        ));
    };
    let Some(expires_at) = grant_expiry_date(base_date, rule.expiry_months) else {
        return Err(LeaveLedgerError::InvalidInput(format!(
            "cannot compute expiry date for base date {base_date}"
        )));
    };

    let granted_minutes = rule.granted_days * rule.day_equivalent_minutes;
    new_entries.push(NewLeaveLedgerEntry {
        user_id: candidate.user_id.clone(),
        leave_type: ANNUAL_LEAVE_TYPE.to_string(),
        kind: LeaveLedgerKind::Grant,
        lot_id: None,
        amount_minutes: granted_minutes,
        day_equivalent_minutes: rule.day_equivalent_minutes,
        granted_at: Some(base_date),
        expires_at: Some(expires_at),
        grant_base_date: Some(base_date),
        leave_request_id: None,
        reason: None,
        created_by: created_by.clone(),
        effective_at: start_of_day_utc(base_date),
    });
    let grant = GrantOutcome {
        user_id: candidate.user_id.clone(),
        lot_id: None,
        tenure_months,
        granted_minutes,
        day_equivalent_minutes: rule.day_equivalent_minutes,
        granted_at: base_date,
        expires_at,
    };
    Ok((
        CandidateOutcome {
            expired,
            grant: Some(grant),
            skip: None,
        },
        new_entries,
    ))
}

struct CandidateOutcome {
    expired: Vec<ExpiryOutcome>,
    grant: Option<GrantOutcome>,
    skip: Option<GrantSkipReason>,
}

fn skip_outcome(
    expired: Vec<ExpiryOutcome>,
    new_entries: Vec<NewLeaveLedgerEntry>,
    reason: GrantSkipReason,
) -> (CandidateOutcome, Vec<NewLeaveLedgerEntry>) {
    (
        CandidateOutcome {
            expired,
            grant: None,
            skip: Some(reason),
        },
        new_entries,
    )
}

/// 冪等な expire 補記（付与可否に関わらず全対象者に対して行う）。
fn expiration_entries(
    candidate: &GrantCandidate,
    balance: &LeaveBalance,
    created_by: &Option<String>,
) -> (Vec<ExpiryOutcome>, Vec<NewLeaveLedgerEntry>) {
    let mut expired = Vec::new();
    let mut new_entries = Vec::new();
    for draft in pending_expirations(balance) {
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
            created_by: created_by.clone(),
            effective_at: start_of_day_utc(draft.expires_at),
        });
    }
    (expired, new_entries)
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
            if lot.remaining_minutes + command.amount_minutes < 0 {
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
