use chrono::NaiveDate;
use timekeeper_domain::leave_ledger::{
    allocate_fifo, annual_obligations, derive_balance, grant_expiry_date, pending_expirations,
    scheduled_grant_tenure_months, select_grant_rule, FifoAllocationError, LeaveGrantRule,
    LeaveLedgerEvent, LeaveLedgerKind, LeaveObligationRule, ObligationStatus,
};

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("valid date")
}

fn grant(
    lot_id: &str,
    granted_at: NaiveDate,
    expires_at: NaiveDate,
    minutes: i64,
) -> LeaveLedgerEvent {
    LeaveLedgerEvent {
        lot_id: lot_id.to_string(),
        kind: LeaveLedgerKind::Grant,
        amount_minutes: minutes,
        day_equivalent_minutes: 480,
        granted_at: Some(granted_at),
        expires_at: Some(expires_at),
        grant_base_date: Some(granted_at),
        effective_on: granted_at,
    }
}

fn event(lot_id: &str, kind: LeaveLedgerKind, minutes: i64, on: NaiveDate) -> LeaveLedgerEvent {
    LeaveLedgerEvent {
        lot_id: lot_id.to_string(),
        kind,
        amount_minutes: minutes,
        day_equivalent_minutes: 480,
        granted_at: None,
        expires_at: None,
        grant_base_date: None,
        effective_on: on,
    }
}

#[test]
fn empty_ledger_derives_zero_balance() {
    let balance = derive_balance(&[], date(2026, 7, 5));
    assert_eq!(balance.available_minutes, 0);
    assert!(balance.lots.is_empty());
}

#[test]
fn grant_is_available_until_the_day_before_expiry() {
    let events = vec![grant("lot-1", date(2024, 7, 1), date(2026, 7, 1), 4800)];

    let day_before = derive_balance(&events, date(2026, 6, 30));
    assert_eq!(day_before.available_minutes, 4800);

    let on_expiry = derive_balance(&events, date(2026, 7, 1));
    assert_eq!(on_expiry.available_minutes, 0);
    assert!(on_expiry.lots[0].is_expired(date(2026, 7, 1)));
}

#[test]
fn balance_ignores_events_after_as_of() {
    let events = vec![
        grant("lot-1", date(2026, 1, 1), date(2028, 1, 1), 4800),
        grant("lot-2", date(2026, 8, 1), date(2028, 8, 1), 5280),
    ];
    let balance = derive_balance(&events, date(2026, 7, 5));
    assert_eq!(balance.available_minutes, 4800);
    assert_eq!(balance.lots.len(), 1);
}

#[test]
fn consume_and_release_net_out_within_a_lot() {
    let events = vec![
        grant("lot-1", date(2026, 1, 1), date(2028, 1, 1), 4800),
        event("lot-1", LeaveLedgerKind::Consume, -480, date(2026, 2, 1)),
        event("lot-1", LeaveLedgerKind::Consume, -960, date(2026, 3, 1)),
        event("lot-1", LeaveLedgerKind::Release, 480, date(2026, 3, 5)),
    ];
    let balance = derive_balance(&events, date(2026, 7, 5));
    assert_eq!(balance.available_minutes, 4800 - 480 - 960 + 480);
}

#[test]
fn lots_are_ordered_fifo_by_expiry_then_grant_date_then_lot_id() {
    let events = vec![
        grant("lot-c", date(2025, 1, 1), date(2027, 1, 1), 480),
        grant("lot-a", date(2026, 1, 1), date(2028, 1, 1), 480),
        // Same expiry as lot-c but granted later.
        LeaveLedgerEvent {
            expires_at: Some(date(2027, 1, 1)),
            ..grant("lot-b", date(2025, 6, 1), date(2027, 1, 1), 480)
        },
        // Same expiry and grant date as lot-c: lot_id breaks the tie.
        grant("lot-d", date(2025, 1, 1), date(2027, 1, 1), 480),
    ];
    let balance = derive_balance(&events, date(2026, 7, 5));
    let order: Vec<&str> = balance.lots.iter().map(|l| l.lot_id.as_str()).collect();
    assert_eq!(order, vec!["lot-c", "lot-d", "lot-b", "lot-a"]);
}

#[test]
fn allocate_fifo_spans_lots_nearest_expiry_first() {
    let events = vec![
        grant("carryover", date(2025, 7, 1), date(2027, 7, 1), 960),
        grant("current", date(2026, 7, 1), date(2028, 7, 1), 4800),
    ];
    let balance = derive_balance(&events, date(2026, 7, 5));
    let allocations = allocate_fifo(&balance, 1440).expect("allocate");
    assert_eq!(allocations.len(), 2);
    assert_eq!(allocations[0].lot_id, "carryover");
    assert_eq!(allocations[0].amount_minutes, 960);
    assert_eq!(allocations[1].lot_id, "current");
    assert_eq!(allocations[1].amount_minutes, 480);
}

#[test]
fn allocate_fifo_rejects_insufficient_balance_and_non_positive_amount() {
    let events = vec![grant("lot-1", date(2026, 1, 1), date(2028, 1, 1), 480)];
    let balance = derive_balance(&events, date(2026, 7, 5));

    assert_eq!(
        allocate_fifo(&balance, 481),
        Err(FifoAllocationError::InsufficientBalance {
            requested_minutes: 481,
            available_minutes: 480,
        })
    );
    assert_eq!(
        allocate_fifo(&balance, 0),
        Err(FifoAllocationError::NonPositiveAmount)
    );
}

#[test]
fn allocate_fifo_skips_expired_lots() {
    let events = vec![
        grant("expired", date(2024, 1, 1), date(2026, 1, 1), 4800),
        grant("active", date(2026, 1, 1), date(2028, 1, 1), 480),
    ];
    let balance = derive_balance(&events, date(2026, 7, 5));
    assert_eq!(balance.available_minutes, 480);
    let allocations = allocate_fifo(&balance, 480).expect("allocate");
    assert_eq!(allocations.len(), 1);
    assert_eq!(allocations[0].lot_id, "active");
}

#[test]
fn pending_expirations_lists_unrecorded_expiries_once() {
    let mut events = vec![
        grant("expired", date(2024, 1, 1), date(2026, 1, 1), 4800),
        event("expired", LeaveLedgerKind::Consume, -480, date(2025, 3, 1)),
        grant("active", date(2026, 1, 1), date(2028, 1, 1), 480),
    ];
    let balance = derive_balance(&events, date(2026, 7, 5));
    let drafts = pending_expirations(&balance);
    assert_eq!(drafts.len(), 1);
    assert_eq!(drafts[0].lot_id, "expired");
    assert_eq!(drafts[0].amount_minutes, -(4800 - 480));
    assert_eq!(drafts[0].expires_at, date(2026, 1, 1));

    // 記帳後は補記対象から消え、残高は変わらない（遅延評価と同値）。
    let mut expire = event(
        "expired",
        LeaveLedgerKind::Expire,
        drafts[0].amount_minutes,
        date(2026, 7, 5),
    );
    expire.effective_on = drafts[0].expires_at;
    events.push(expire);
    let after = derive_balance(&events, date(2026, 7, 5));
    assert!(pending_expirations(&after).is_empty());
    assert_eq!(after.available_minutes, balance.available_minutes);
    assert_eq!(
        after
            .lots
            .iter()
            .find(|l| l.lot_id == "expired")
            .expect("lot")
            .remaining_minutes,
        0
    );
}

#[test]
fn adjust_opening_a_new_lot_behaves_like_a_grant_lot() {
    // 初期移行: adjust で新規ロットを投入する。
    let events = vec![LeaveLedgerEvent {
        lot_id: "migrated".to_string(),
        kind: LeaveLedgerKind::Adjust,
        amount_minutes: 2400,
        day_equivalent_minutes: 480,
        granted_at: Some(date(2025, 10, 1)),
        expires_at: Some(date(2027, 10, 1)),
        grant_base_date: Some(date(2025, 10, 1)),
        effective_on: date(2026, 7, 1),
    }];
    let balance = derive_balance(&events, date(2026, 7, 5));
    assert_eq!(balance.available_minutes, 2400);
    let lot = &balance.lots[0];
    assert_eq!(lot.granted_minutes, 2400);
    assert_eq!(lot.expires_at, Some(date(2027, 10, 1)));
}

#[test]
fn select_grant_rule_picks_highest_tenure_at_or_below() {
    let rules = vec![
        LeaveGrantRule {
            tenure_months: 6,
            granted_days: 10,
            expiry_months: 24,
            day_equivalent_minutes: 480,
        },
        LeaveGrantRule {
            tenure_months: 18,
            granted_days: 11,
            expiry_months: 24,
            day_equivalent_minutes: 480,
        },
        LeaveGrantRule {
            tenure_months: 78,
            granted_days: 20,
            expiry_months: 24,
            day_equivalent_minutes: 480,
        },
    ];
    assert!(select_grant_rule(&rules, 5).is_none());
    assert_eq!(select_grant_rule(&rules, 6).expect("rule").granted_days, 10);
    assert_eq!(
        select_grant_rule(&rules, 17).expect("rule").granted_days,
        10
    );
    assert_eq!(
        select_grant_rule(&rules, 30).expect("rule").granted_days,
        11
    );
    assert_eq!(
        select_grant_rule(&rules, 150).expect("rule").granted_days,
        20
    );
}

#[test]
fn scheduled_grant_tenure_matches_six_months_then_yearly() {
    let hire = date(2020, 4, 1);
    assert_eq!(
        scheduled_grant_tenure_months(hire, date(2020, 10, 1)),
        Some(6)
    );
    assert_eq!(
        scheduled_grant_tenure_months(hire, date(2021, 10, 1)),
        Some(18)
    );
    assert_eq!(
        scheduled_grant_tenure_months(hire, date(2026, 10, 1)),
        Some(78)
    );
    assert_eq!(scheduled_grant_tenure_months(hire, date(2021, 4, 1)), None);
    assert_eq!(scheduled_grant_tenure_months(hire, date(2020, 9, 30)), None);
}

#[test]
fn scheduled_grant_tenure_clamps_month_end_hire_dates() {
    // 8/31 入社 + 6 ヶ月 = 2/28（非閏年の月末クランプ）。
    let hire = date(2020, 8, 31);
    assert_eq!(
        scheduled_grant_tenure_months(hire, date(2021, 2, 28)),
        Some(6)
    );
    assert_eq!(scheduled_grant_tenure_months(hire, date(2021, 3, 2)), None);
}

#[test]
fn grant_expiry_date_adds_expiry_months() {
    assert_eq!(
        grant_expiry_date(date(2026, 7, 1), 24),
        Some(date(2028, 7, 1))
    );
    assert_eq!(
        grant_expiry_date(date(2024, 2, 29), 24),
        Some(date(2026, 2, 28))
    );
}

fn obligation_rule() -> LeaveObligationRule {
    LeaveObligationRule {
        minimum_granted_days: 10,
        required_days: 5,
        window_months: 12,
        warning_lead_days: 90,
    }
}

#[test]
fn obligation_skips_cycles_granted_below_minimum() {
    let events = vec![grant("small", date(2026, 1, 1), date(2028, 1, 1), 480 * 9)];
    assert!(annual_obligations(&events, &obligation_rule(), date(2026, 7, 5)).is_empty());
}

#[test]
fn obligation_counts_consumes_inside_the_window_only() {
    let base = date(2026, 1, 1);
    let events = vec![
        grant("lot-1", base, date(2028, 1, 1), 4800),
        event(
            "lot-1",
            LeaveLedgerKind::Consume,
            -480 * 2,
            date(2026, 3, 1),
        ),
        // window 外（翌サイクル）の消化は数えない。
        event("lot-1", LeaveLedgerKind::Consume, -480, date(2027, 2, 1)),
    ];
    let windows = annual_obligations(&events, &obligation_rule(), date(2027, 3, 1));
    assert_eq!(windows.len(), 1);
    let window = &windows[0];
    assert_eq!(window.window_end, date(2027, 1, 1));
    assert_eq!(window.taken_minutes, 480 * 2);
    // window は過ぎており未達 → 残期間で達成不可能。
    assert_eq!(window.status, ObligationStatus::AtRisk);
}

#[test]
fn obligation_statuses_cover_ok_warning_at_risk_fulfilled() {
    let base = date(2026, 1, 1);
    let rule = obligation_rule();
    let granted = vec![grant("lot-1", base, date(2028, 1, 1), 4800)];

    // 残り 300 日超・未消化 → ok
    let ok = annual_obligations(&granted, &rule, date(2026, 2, 1));
    assert_eq!(ok[0].status, ObligationStatus::Ok);

    // window 終了まで 90 日以内 → warning
    let warning = annual_obligations(&granted, &rule, date(2026, 11, 1));
    assert_eq!(warning[0].status, ObligationStatus::Warning);

    // 残り 2 日で 5 日必要 → at_risk
    let at_risk = annual_obligations(&granted, &rule, date(2026, 12, 30));
    assert_eq!(at_risk[0].status, ObligationStatus::AtRisk);

    // 5 日消化済み → fulfilled
    let mut fulfilled_events = granted.clone();
    fulfilled_events.push(event(
        "lot-1",
        LeaveLedgerKind::Consume,
        -480 * 5,
        date(2026, 5, 1),
    ));
    let fulfilled = annual_obligations(&fulfilled_events, &rule, date(2026, 6, 1));
    assert_eq!(fulfilled[0].status, ObligationStatus::Fulfilled);
}

#[test]
fn obligation_release_cancels_consumed_days() {
    let base = date(2026, 1, 1);
    let events = vec![
        grant("lot-1", base, date(2028, 1, 1), 4800),
        event(
            "lot-1",
            LeaveLedgerKind::Consume,
            -480 * 5,
            date(2026, 5, 1),
        ),
        event(
            "lot-1",
            LeaveLedgerKind::Release,
            480 * 2,
            date(2026, 5, 10),
        ),
    ];
    let windows = annual_obligations(&events, &obligation_rule(), date(2026, 6, 1));
    assert_eq!(windows[0].taken_minutes, 480 * 3);
    assert_ne!(windows[0].status, ObligationStatus::Fulfilled);
}

#[test]
fn obligation_merges_multiple_grants_on_the_same_base_date() {
    let base = date(2026, 1, 1);
    let events = vec![
        grant("lot-1", base, date(2028, 1, 1), 480 * 6),
        grant("lot-2", base, date(2028, 1, 1), 480 * 6),
    ];
    let windows = annual_obligations(&events, &obligation_rule(), date(2026, 2, 1));
    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0].granted_minutes, 480 * 12);
}

/// M-2: `remaining_minutes` の加算はもう素朴な `+=` ではなく `checked_add` を
/// 使う。この不変条件は「単一イベントの `amount_minutes` は contract 層
/// （i32 range）と DB 列（`INTEGER`）で常に i32 範囲に収まる」という上流の
/// 保証に依存しており、破られた場合は静かにラップアラウンドさせず
/// fail-fast する（本来ここまで到達しないはずの異常系）。
#[test]
#[should_panic(expected = "i64 accumulation overflow")]
fn derive_balance_panics_on_amount_minutes_i64_overflow() {
    let events = vec![
        grant("lot-1", date(2025, 1, 1), date(2027, 1, 1), i64::MAX),
        event("lot-1", LeaveLedgerKind::Adjust, i64::MAX, date(2025, 2, 1)),
    ];
    derive_balance(&events, date(2026, 7, 5));
}
