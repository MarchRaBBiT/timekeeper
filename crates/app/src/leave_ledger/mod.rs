//! 有給付与・残高台帳の use case（T-04）。
//!
//! - `GetLeaveBalance`: 残高・時効予定・年5日義務の read-model 導出（保存しない）
//! - `RunLeaveGrants`: 管理者起動の付与実行（dry-run 付き）+ 冪等な expire 補記
//! - `AdjustLeaveLedger`: 初期移行・手動調整の adjust 投入（dry-run 照合付き）
//! - `SetHireDate`: 付与基準日の起点となる入社日の投入
//!
//! 純ロジックは `timekeeper_domain::leave_ledger`、SQL は infra 層に置く。
//!
//! M-7: 800 行超過のため use case 単位で submodule へ分割する。
//! エラー型・repository trait・共通型はこの `mod.rs` に置き、各 submodule の
//! 公開項目を `pub use` で再輸出することで、既存の
//! `timekeeper_app::leave_ledger::X` という import パスを変更しない。
//! - `balance`: `GetLeaveBalance`（残高 read-model）
//! - `consume`: 有給申請の消化 (`consume`) ・取消時解放 (`release`) entries 構築
//! - `grants`: `RunLeaveGrants`（付与バッチ）
//! - `adjust`: `AdjustLeaveLedger`（手動調整）

use std::future::Future;
use std::pin::Pin;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use thiserror::Error;
use timekeeper_domain::leave_ledger::{
    LeaveGrantRule, LeaveLedgerEvent, LeaveLedgerKind, LeaveObligationRule,
};

mod adjust;
mod balance;
mod consume;
mod grants;

pub use adjust::{AdjustLeaveLedger, AdjustLeaveLedgerCommand, AdjustLeaveLedgerReport};
pub use balance::{GetLeaveBalance, GetLeaveBalanceCommand, LeaveBalanceView};
pub use consume::{
    build_annual_leave_consume_entries, build_annual_leave_release_entries,
    build_leave_consume_entries_for_minutes, ensure_annual_leave_request_has_balance,
    ensure_leave_request_minutes_have_balance, AnnualLeaveRequestLedgerCommand,
};
pub use grants::{
    ExpiryOutcome, GrantFailure, GrantOutcome, GrantSkipReason, RunLeaveGrants,
    RunLeaveGrantsCommand, RunLeaveGrantsReport,
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
    pub obligation_minutes: i64,
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
            obligation_minutes: self.obligation_minutes,
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
    pub obligation_minutes: i64,
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

/// 台帳の永続化済み entries を、純ロジック側（`timekeeper_domain::leave_ledger`）
/// が扱う `LeaveLedgerEvent` へ変換する。`balance` / `consume` / `grants` /
/// `adjust` の各 submodule から共有される。
fn stored_entries_to_events(entries: &[StoredLeaveLedgerEntry]) -> Vec<LeaveLedgerEvent> {
    entries
        .iter()
        .map(StoredLeaveLedgerEntry::to_domain_event)
        .collect()
}

/// `consume` / `grants` submodule から共有される、日付を UTC 0 時の
/// `DateTime<Utc>` へ変換するヘルパー。
fn start_of_day_utc(date: NaiveDate) -> DateTime<Utc> {
    DateTime::<Utc>::from_naive_utc_and_offset(date.and_hms_opt(0, 0, 0).expect("midnight"), Utc)
}
