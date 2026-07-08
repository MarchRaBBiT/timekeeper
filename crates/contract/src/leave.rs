//! 有給付与・残高台帳の API DTO（T-04）。
//!
//! 数量は分（integer minutes）が正で、日数は `day_equivalent_minutes` から
//! 導出した表示値（docs/design-docs/leave-entitlement.md）。

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

pub const LEAVE_BALANCE_INSUFFICIENT_CODE: &str = "LEAVE_BALANCE_INSUFFICIENT";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum LeaveLedgerKind {
    Grant,
    Consume,
    Release,
    Expire,
    Adjust,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum LeaveObligationStatus {
    Ok,
    Warning,
    AtRisk,
    Fulfilled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, IntoParams)]
pub struct LeaveBalanceQuery {
    /// 残高を導出する基準日（省略時はサーバー現在日）。
    pub as_of: Option<NaiveDate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct LeaveLotResponse {
    pub lot_id: String,
    pub granted_at: Option<NaiveDate>,
    pub expires_at: Option<NaiveDate>,
    pub grant_base_date: Option<NaiveDate>,
    pub day_equivalent_minutes: i64,
    pub granted_minutes: i64,
    pub remaining_minutes: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct LeaveExpiryScheduleResponse {
    pub lot_id: String,
    pub expires_at: NaiveDate,
    pub remaining_minutes: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct LeaveObligationWindowResponse {
    pub grant_base_date: NaiveDate,
    /// window 終端（exclusive）。この日の前日までが対象期間。
    pub window_end: NaiveDate,
    pub day_equivalent_minutes: i64,
    pub granted_minutes: i64,
    pub required_minutes: i64,
    pub taken_minutes: i64,
    pub required_days: f64,
    pub taken_days: f64,
    pub status: LeaveObligationStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct LeaveBalanceResponse {
    pub user_id: String,
    pub leave_type: String,
    pub as_of: NaiveDate,
    /// 消化に使える残分（時効控除済み）。
    pub available_minutes: i64,
    /// 表示用の日数換算（ロットごとの `day_equivalent_minutes` で換算した合計）。
    pub available_days: f64,
    /// 未失効・残分ありのロット（FIFO = 時効の近い順）。
    pub active_lots: Vec<LeaveLotResponse>,
    /// 時効予定（`active_lots` の時効日順ビュー）。
    pub upcoming_expiries: Vec<LeaveExpiryScheduleResponse>,
    /// 年5日取得義務の消化状況（義務ルール未設定時は空）。
    pub obligations: Vec<LeaveObligationWindowResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate, ToSchema)]
pub struct LeaveGrantRunRequest {
    /// 付与基準日。入社日 + 6 ヶ月 + 12k ヶ月がこの日に一致するユーザーが対象。
    pub base_date: NaiveDate,
    /// true なら台帳へ書き込まず結果だけ返す。
    #[serde(default)]
    pub dry_run: bool,
    /// 対象を明示する場合のユーザー ID リスト（省略時は全ユーザー）。
    #[serde(default)]
    pub user_ids: Option<Vec<String>>,
    /// 除外するユーザー ID リスト（出勤率 8 割未満などの運用判断）。
    #[serde(default)]
    pub exclude_user_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct LeaveGrantResultResponse {
    pub user_id: String,
    /// dry-run では未採番のため None。
    pub lot_id: Option<String>,
    pub tenure_months: i64,
    pub granted_minutes: i64,
    pub granted_days: i64,
    pub day_equivalent_minutes: i64,
    pub granted_at: NaiveDate,
    pub expires_at: NaiveDate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum LeaveGrantSkipReason {
    HireDateNotSet,
    NotDue,
    AlreadyGranted,
    NoMatchingRule,
    Excluded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct LeaveGrantSkipResponse {
    pub user_id: String,
    pub reason: LeaveGrantSkipReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct LeaveExpiryBackfillResponse {
    pub user_id: String,
    pub lot_id: String,
    /// 負値（失効した残分）。
    pub amount_minutes: i64,
    pub expires_at: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct LeaveGrantRunResponse {
    pub base_date: NaiveDate,
    pub dry_run: bool,
    pub granted: Vec<LeaveGrantResultResponse>,
    pub skipped: Vec<LeaveGrantSkipResponse>,
    /// 冪等な expire 補記（遅延評価済みのため残高は変わらない）。
    pub expired: Vec<LeaveExpiryBackfillResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate, ToSchema)]
pub struct LeaveLedgerAdjustRequest {
    pub user_id: String,
    /// 符号付き分。新規ロット投入（初期移行）は正、控除は負。
    pub amount_minutes: i64,
    /// 既存ロットの補正時に指定。省略時は新規ロットを採番する。
    #[serde(default)]
    pub lot_id: Option<String>,
    /// 新規ロット時に必須。1 日 = N 分の換算値。
    #[serde(default)]
    pub day_equivalent_minutes: Option<i64>,
    /// 新規ロット時に必須。ロットの付与日。
    #[serde(default)]
    pub granted_at: Option<NaiveDate>,
    /// 新規ロット時に必須。時効日。
    #[serde(default)]
    pub expires_at: Option<NaiveDate>,
    /// 年5日義務 window の起算日（新規ロット時のみ意味を持つ）。
    #[serde(default)]
    pub grant_base_date: Option<NaiveDate>,
    /// 監査用の理由（必須）。
    #[validate(length(min = 1, max = 500))]
    pub reason: String,
    /// true なら書き込まず、投入後の導出残高だけ返す（初期移行の照合用）。
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct LeaveLedgerEntryResponse {
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct LeaveLedgerAdjustResponse {
    pub dry_run: bool,
    /// dry-run では書き込まれないため None。
    pub entry: Option<LeaveLedgerEntryResponse>,
    /// 投入（相当）後の導出残高（分）。初期移行の dry-run 照合に使う。
    pub balance_after_minutes: i64,
    pub balance_after_days: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Validate, ToSchema)]
pub struct SetHireDateRequest {
    /// 付与基準日の起点となる入社日。
    pub hire_date: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct HireDateResponse {
    pub user_id: String,
    pub hire_date: NaiveDate,
}
