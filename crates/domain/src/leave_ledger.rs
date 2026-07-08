//! 有給休暇台帳（append-only ledger）の純ロジック。
//!
//! 残高は保存せず、イベント列から導出する（docs/design-docs/leave-entitlement.md）。
//! - ロット導出と FIFO（時効の近い付与ロットから消化）順序付け
//! - 時効の遅延評価（`expires_at <= as_of` のロットは残高から控除）
//! - 冪等な `expire` 補記のためのドラフト計算
//! - 付与ルール（勤続年数テーブル）選択と基準日判定
//! - 年5日取得義務の window 判定
//!
//! 法定値（付与日数・時効月数・義務日数）はここに持たず、呼び出し側が
//! マスタから渡す（共通作業規約 8）。

use chrono::{Months, NaiveDate};

/// 台帳イベント種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LeaveLedgerKind {
    Grant,
    Consume,
    Release,
    Expire,
    Adjust,
}

/// 残高導出に必要な台帳イベントの射影。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaveLedgerEvent {
    pub lot_id: String,
    pub kind: LeaveLedgerKind,
    /// 符号付き分。grant/release/正 adjust は正、consume/expire/負 adjust は負。
    pub amount_minutes: i64,
    /// このロットの 1 日 = N 分換算（grant 時に固定）。
    pub day_equivalent_minutes: i64,
    pub granted_at: Option<NaiveDate>,
    pub expires_at: Option<NaiveDate>,
    pub grant_base_date: Option<NaiveDate>,
    /// 時点集計キー（`effective_at` の日付部）。
    pub effective_on: NaiveDate,
}

impl LeaveLedgerEvent {
    /// ロットを新設するイベント（`grant` または新規ロット `adjust`）か。
    fn opens_lot(&self) -> bool {
        match self.kind {
            LeaveLedgerKind::Grant => true,
            LeaveLedgerKind::Adjust => self.granted_at.is_some() && self.amount_minutes > 0,
            _ => false,
        }
    }
}

/// 導出された付与ロット。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaveLot {
    pub lot_id: String,
    pub granted_at: Option<NaiveDate>,
    pub expires_at: Option<NaiveDate>,
    pub grant_base_date: Option<NaiveDate>,
    pub day_equivalent_minutes: i64,
    /// ロット新設イベント（grant / 新規ロット adjust）の合計分。
    pub granted_minutes: i64,
    /// 符号付き全イベント合計。`expire` 記帳済みなら 0 になる。
    pub remaining_minutes: i64,
    pub has_expire_event: bool,
}

impl LeaveLot {
    /// 時効の遅延評価: `expires_at <= as_of` は失効済みとみなす。
    pub fn is_expired(&self, as_of: NaiveDate) -> bool {
        self.expires_at.is_some_and(|date| date <= as_of)
    }

    /// `as_of` 時点で消化に使える分（失効ロットは 0、負残高は 0 に切り上げ）。
    pub fn available_minutes(&self, as_of: NaiveDate) -> i64 {
        if self.is_expired(as_of) {
            0
        } else {
            self.remaining_minutes.max(0)
        }
    }
}

/// `as_of` 時点の導出残高。`lots` は FIFO（時効の近い順）で並ぶ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaveBalance {
    pub as_of: NaiveDate,
    pub available_minutes: i64,
    pub lots: Vec<LeaveLot>,
}

impl LeaveBalance {
    /// 未失効かつ残分のあるロット（FIFO 順）。
    pub fn active_lots(&self) -> impl Iterator<Item = &LeaveLot> {
        self.lots
            .iter()
            .filter(|lot| !lot.is_expired(self.as_of) && lot.remaining_minutes > 0)
    }
}

/// イベント列から `as_of` 時点の残高を導出する。
///
/// `effective_on > as_of` のイベントは無視する（時点再現性）。
pub fn derive_balance(events: &[LeaveLedgerEvent], as_of: NaiveDate) -> LeaveBalance {
    let mut lots: Vec<LeaveLot> = Vec::new();
    for event in events {
        if event.effective_on > as_of {
            continue;
        }
        let lot = match lots.iter_mut().find(|lot| lot.lot_id == event.lot_id) {
            Some(existing) => existing,
            None => {
                lots.push(LeaveLot {
                    lot_id: event.lot_id.clone(),
                    granted_at: None,
                    expires_at: None,
                    grant_base_date: None,
                    day_equivalent_minutes: event.day_equivalent_minutes,
                    granted_minutes: 0,
                    remaining_minutes: 0,
                    has_expire_event: false,
                });
                lots.last_mut().expect("lot just pushed")
            }
        };
        lot.remaining_minutes += event.amount_minutes;
        if event.kind == LeaveLedgerKind::Expire {
            lot.has_expire_event = true;
        }
        if event.opens_lot() {
            lot.granted_minutes += event.amount_minutes;
            lot.granted_at = lot.granted_at.or(event.granted_at);
            lot.expires_at = lot.expires_at.or(event.expires_at);
            lot.grant_base_date = lot.grant_base_date.or(event.grant_base_date);
            lot.day_equivalent_minutes = event.day_equivalent_minutes;
        }
    }
    sort_fifo(&mut lots);
    let available_minutes = lots.iter().map(|lot| lot.available_minutes(as_of)).sum();
    LeaveBalance {
        as_of,
        available_minutes,
        lots,
    }
}

/// FIFO 順: 時効が近い順 → 付与日が古い順 → `lot_id` 昇順（決定的）。
fn sort_fifo(lots: &mut [LeaveLot]) {
    lots.sort_by(|a, b| {
        let a_key = (
            a.expires_at.unwrap_or(NaiveDate::MAX),
            a.granted_at.unwrap_or(NaiveDate::MAX),
        );
        let b_key = (
            b.expires_at.unwrap_or(NaiveDate::MAX),
            b.granted_at.unwrap_or(NaiveDate::MAX),
        );
        a_key.cmp(&b_key).then_with(|| a.lot_id.cmp(&b.lot_id))
    });
}

/// FIFO 引当の 1 ロット分。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LotAllocation {
    pub lot_id: String,
    pub amount_minutes: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FifoAllocationError {
    #[error("allocation amount must be positive")]
    NonPositiveAmount,
    #[error(
        "insufficient leave balance: requested {requested_minutes}, available {available_minutes}"
    )]
    InsufficientBalance {
        requested_minutes: i64,
        available_minutes: i64,
    },
}

/// `amount_minutes` を FIFO 順に引き当てる（T-05 の consume がこの結果を使う）。
pub fn allocate_fifo(
    balance: &LeaveBalance,
    amount_minutes: i64,
) -> Result<Vec<LotAllocation>, FifoAllocationError> {
    if amount_minutes <= 0 {
        return Err(FifoAllocationError::NonPositiveAmount);
    }
    if amount_minutes > balance.available_minutes {
        return Err(FifoAllocationError::InsufficientBalance {
            requested_minutes: amount_minutes,
            available_minutes: balance.available_minutes,
        });
    }
    let mut rest = amount_minutes;
    let mut allocations = Vec::new();
    for lot in balance.active_lots() {
        if rest == 0 {
            break;
        }
        let take = rest.min(lot.available_minutes(balance.as_of));
        if take > 0 {
            allocations.push(LotAllocation {
                lot_id: lot.lot_id.clone(),
                amount_minutes: take,
            });
            rest -= take;
        }
    }
    debug_assert_eq!(rest, 0, "active lots must cover the checked total");
    Ok(allocations)
}

/// 冪等な `expire` 補記のドラフト。`amount_minutes` は負値。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpirationDraft {
    pub lot_id: String,
    pub amount_minutes: i64,
    pub expires_at: NaiveDate,
    pub day_equivalent_minutes: i64,
}

/// `as_of` までに失効したのに `expire` 未記帳のロットを列挙する。
///
/// 残高導出は遅延評価で既に控除済みなので、この補記が遅れても残高は正しい。
pub fn pending_expirations(balance: &LeaveBalance) -> Vec<ExpirationDraft> {
    balance
        .lots
        .iter()
        .filter(|lot| {
            lot.is_expired(balance.as_of) && lot.remaining_minutes > 0 && !lot.has_expire_event
        })
        .map(|lot| ExpirationDraft {
            lot_id: lot.lot_id.clone(),
            amount_minutes: -lot.remaining_minutes,
            expires_at: lot.expires_at.expect("expired lot has expires_at"),
            day_equivalent_minutes: lot.day_equivalent_minutes,
        })
        .collect()
}

/// 付与ルールマスタの 1 行（standard accrual）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaveGrantRule {
    pub tenure_months: i64,
    pub granted_days: i64,
    pub expiry_months: i64,
    pub day_equivalent_minutes: i64,
}

/// 勤続月数に適用される付与ルール（`tenure_months <= 勤続月数` の最大行）を選ぶ。
pub fn select_grant_rule(rules: &[LeaveGrantRule], tenure_months: i64) -> Option<&LeaveGrantRule> {
    rules
        .iter()
        .filter(|rule| rule.tenure_months <= tenure_months)
        .max_by_key(|rule| rule.tenure_months)
}

/// `base_date` が入社日起算の付与基準日（入社 + 6 ヶ月、以後 1 年ごと）に一致するなら
/// その時点の勤続月数を返す。月末入社は暦月加算（月末クランプ）で決定的に扱う。
pub fn scheduled_grant_tenure_months(hire_date: NaiveDate, base_date: NaiveDate) -> Option<i64> {
    let mut months: u32 = 6;
    loop {
        let candidate = hire_date.checked_add_months(Months::new(months))?;
        if candidate == base_date {
            return Some(i64::from(months));
        }
        if candidate > base_date {
            return None;
        }
        months = months.checked_add(12)?;
    }
}

/// 付与日と時効月数から時効日を算出する（月末クランプ）。
pub fn grant_expiry_date(granted_at: NaiveDate, expiry_months: i64) -> Option<NaiveDate> {
    let months = u32::try_from(expiry_months).ok()?;
    granted_at.checked_add_months(Months::new(months))
}

/// 年5日取得義務のルール（マスタ由来）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaveObligationRule {
    /// この日数以上付与された基準日サイクルが義務の対象になる。
    pub minimum_granted_days: i64,
    /// window 内に取得すべき日数。
    pub required_days: i64,
    /// 基準日からの window 長（月）。
    pub window_months: i64,
    /// window 終了がこの日数以内に迫ったら warning。
    pub warning_lead_days: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObligationStatus {
    Ok,
    Warning,
    AtRisk,
    Fulfilled,
}

/// 基準日サイクルごとの義務判定結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObligationWindow {
    pub grant_base_date: NaiveDate,
    /// window 終端（exclusive）。
    pub window_end: NaiveDate,
    pub day_equivalent_minutes: i64,
    pub granted_minutes: i64,
    pub required_minutes: i64,
    pub taken_minutes: i64,
    pub status: ObligationStatus,
}

/// 年5日取得義務の判定。`grant_base_date` を持つロット新設イベントを
/// サイクルへ集約し、window 内の consume（release で打ち消し済みは除く）を
/// 取得分として数える。
pub fn annual_obligations(
    events: &[LeaveLedgerEvent],
    rule: &LeaveObligationRule,
    as_of: NaiveDate,
) -> Vec<ObligationWindow> {
    let mut cycles: Vec<(NaiveDate, i64, i64)> = Vec::new(); // (base, granted, day_equivalent)
    for event in events {
        if !event.opens_lot() || event.effective_on > as_of {
            continue;
        }
        let Some(base) = event.grant_base_date else {
            continue;
        };
        match cycles.iter_mut().find(|(cycle, _, _)| *cycle == base) {
            Some((_, granted, _)) => *granted += event.amount_minutes,
            None => cycles.push((base, event.amount_minutes, event.day_equivalent_minutes)),
        }
    }
    cycles.sort_by_key(|(base, _, _)| *base);

    let mut windows = Vec::new();
    for (base, granted_minutes, day_equivalent_minutes) in cycles {
        if day_equivalent_minutes <= 0
            || granted_minutes < rule.minimum_granted_days * day_equivalent_minutes
        {
            continue;
        }
        let Some(window_end) = add_months(base, rule.window_months) else {
            continue;
        };
        let taken_minutes: i64 = events
            .iter()
            .filter(|event| {
                matches!(
                    event.kind,
                    LeaveLedgerKind::Consume | LeaveLedgerKind::Release
                ) && event.effective_on >= base
                    && event.effective_on < window_end
                    && event.effective_on <= as_of
            })
            .map(|event| -event.amount_minutes)
            .sum();
        let required_minutes = rule.required_days * day_equivalent_minutes;
        let status = obligation_status(
            required_minutes,
            taken_minutes,
            day_equivalent_minutes,
            window_end,
            as_of,
            rule.warning_lead_days,
        );
        windows.push(ObligationWindow {
            grant_base_date: base,
            window_end,
            day_equivalent_minutes,
            granted_minutes,
            required_minutes,
            taken_minutes,
            status,
        });
    }
    windows
}

fn obligation_status(
    required_minutes: i64,
    taken_minutes: i64,
    day_equivalent_minutes: i64,
    window_end: NaiveDate,
    as_of: NaiveDate,
    warning_lead_days: i64,
) -> ObligationStatus {
    if taken_minutes >= required_minutes {
        return ObligationStatus::Fulfilled;
    }
    let remaining_minutes = required_minutes - taken_minutes;
    let remaining_required_days =
        (remaining_minutes + day_equivalent_minutes - 1) / day_equivalent_minutes;
    let remaining_calendar_days = (window_end - as_of).num_days();
    if remaining_required_days > remaining_calendar_days {
        return ObligationStatus::AtRisk;
    }
    if remaining_calendar_days <= warning_lead_days {
        return ObligationStatus::Warning;
    }
    ObligationStatus::Ok
}

fn add_months(date: NaiveDate, months: i64) -> Option<NaiveDate> {
    let months = u32::try_from(months).ok()?;
    date.checked_add_months(Months::new(months))
}
