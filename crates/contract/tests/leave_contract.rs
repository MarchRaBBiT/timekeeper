use chrono::{NaiveDate, TimeZone, Utc};
use timekeeper_contract::leave::{
    HireDateResponse, LeaveBalanceResponse, LeaveExpiryScheduleResponse, LeaveGrantResultResponse,
    LeaveGrantRunRequest, LeaveGrantRunResponse, LeaveGrantSkipReason, LeaveGrantSkipResponse,
    LeaveLedgerAdjustRequest, LeaveLedgerAdjustResponse, LeaveLedgerEntryResponse, LeaveLedgerKind,
    LeaveLotResponse, LeaveObligationStatus, LeaveObligationWindowResponse, SetHireDateRequest,
    LEAVE_BALANCE_INSUFFICIENT_CODE,
};
use validator::Validate;

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("valid date")
}

#[test]
fn ledger_kind_uses_snake_case_wire_format() {
    assert_eq!(
        serde_json::to_value(LeaveLedgerKind::Grant).expect("serialize"),
        "grant"
    );
    let parsed: LeaveLedgerKind =
        serde_json::from_value(serde_json::json!("adjust")).expect("deserialize");
    assert_eq!(parsed, LeaveLedgerKind::Adjust);
}

#[test]
fn obligation_status_uses_snake_case_wire_format() {
    assert_eq!(
        serde_json::to_value(LeaveObligationStatus::AtRisk).expect("serialize"),
        "at_risk"
    );
    let parsed: LeaveObligationStatus =
        serde_json::from_value(serde_json::json!("fulfilled")).expect("deserialize");
    assert_eq!(parsed, LeaveObligationStatus::Fulfilled);
}

#[test]
fn insufficient_balance_error_code_is_contract_fixed() {
    assert_eq!(
        LEAVE_BALANCE_INSUFFICIENT_CODE,
        "LEAVE_BALANCE_INSUFFICIENT"
    );
}

#[test]
fn grant_skip_reason_uses_snake_case_wire_format() {
    assert_eq!(
        serde_json::to_value(LeaveGrantSkipReason::HireDateNotSet).expect("serialize"),
        "hire_date_not_set"
    );
}

#[test]
fn grant_run_request_defaults_optional_fields() {
    let parsed: LeaveGrantRunRequest =
        serde_json::from_value(serde_json::json!({ "base_date": "2026-07-01" }))
            .expect("deserialize minimal request");
    assert_eq!(parsed.base_date, date(2026, 7, 1));
    assert!(!parsed.dry_run);
    assert!(parsed.user_ids.is_none());
    assert!(parsed.exclude_user_ids.is_none());
}

#[test]
fn grant_run_response_roundtrips() {
    let response = LeaveGrantRunResponse {
        base_date: date(2026, 7, 1),
        dry_run: true,
        granted: vec![LeaveGrantResultResponse {
            user_id: "user-1".to_string(),
            lot_id: None,
            tenure_months: 6,
            granted_minutes: 4800,
            granted_days: 10,
            day_equivalent_minutes: 480,
            granted_at: date(2026, 7, 1),
            expires_at: date(2028, 7, 1),
        }],
        skipped: vec![LeaveGrantSkipResponse {
            user_id: "user-2".to_string(),
            reason: LeaveGrantSkipReason::NotDue,
        }],
        expired: Vec::new(),
    };
    let json = serde_json::to_value(&response).expect("serialize");
    let back: LeaveGrantRunResponse = serde_json::from_value(json).expect("deserialize");
    assert_eq!(back, response);
}

#[test]
fn balance_response_roundtrips() {
    let response = LeaveBalanceResponse {
        user_id: "user-1".to_string(),
        leave_type: "annual".to_string(),
        as_of: date(2026, 7, 5),
        available_minutes: 5280,
        available_days: 11.0,
        active_lots: vec![LeaveLotResponse {
            lot_id: "lot-1".to_string(),
            granted_at: Some(date(2026, 7, 1)),
            expires_at: Some(date(2028, 7, 1)),
            grant_base_date: Some(date(2026, 7, 1)),
            day_equivalent_minutes: 480,
            granted_minutes: 5280,
            remaining_minutes: 5280,
        }],
        upcoming_expiries: vec![LeaveExpiryScheduleResponse {
            lot_id: "lot-1".to_string(),
            expires_at: date(2028, 7, 1),
            remaining_minutes: 5280,
        }],
        obligations: vec![LeaveObligationWindowResponse {
            grant_base_date: date(2026, 7, 1),
            window_end: date(2027, 7, 1),
            day_equivalent_minutes: 480,
            granted_minutes: 5280,
            required_minutes: 2400,
            taken_minutes: 480,
            required_days: 5.0,
            taken_days: 1.0,
            status: LeaveObligationStatus::Ok,
        }],
    };
    let json = serde_json::to_value(&response).expect("serialize");
    assert_eq!(json["obligations"][0]["status"], "ok");
    let back: LeaveBalanceResponse = serde_json::from_value(json).expect("deserialize");
    assert_eq!(back, response);
}

#[test]
fn adjust_request_requires_reason_and_defaults_flags() {
    let parsed: LeaveLedgerAdjustRequest = serde_json::from_value(serde_json::json!({
        "user_id": "user-1",
        "amount_minutes": 2400,
        "reason": "initial migration"
    }))
    .expect("deserialize minimal adjust");
    assert!(!parsed.dry_run);
    assert!(parsed.lot_id.is_none());
    assert!(parsed.validate().is_ok());

    let empty_reason = LeaveLedgerAdjustRequest {
        reason: String::new(),
        ..parsed
    };
    assert!(empty_reason.validate().is_err());
}

/// M-2: `amount_minutes` は DB 列（`INTEGER` = i32）へ収まる範囲でなければ
/// ならない。`dry_run=true` は DB の `i32::try_from` を経由しないため、
/// 契約層でのバリデーションが唯一のガードになる。
#[test]
fn adjust_request_rejects_amount_minutes_outside_i32_range() {
    let base = LeaveLedgerAdjustRequest {
        user_id: "user-1".to_string(),
        amount_minutes: 2400,
        lot_id: None,
        day_equivalent_minutes: None,
        granted_at: None,
        expires_at: None,
        grant_base_date: None,
        reason: "initial migration".to_string(),
        dry_run: true,
    };
    assert!(base.validate().is_ok());

    let too_low = LeaveLedgerAdjustRequest {
        amount_minutes: i64::MIN,
        ..base.clone()
    };
    assert!(too_low.validate().is_err());

    let too_high = LeaveLedgerAdjustRequest {
        amount_minutes: i64::MAX,
        ..base.clone()
    };
    assert!(too_high.validate().is_err());

    let min_boundary = LeaveLedgerAdjustRequest {
        amount_minutes: -2_147_483_648,
        ..base.clone()
    };
    assert!(min_boundary.validate().is_ok());

    let max_boundary = LeaveLedgerAdjustRequest {
        amount_minutes: 2_147_483_647,
        ..base
    };
    assert!(max_boundary.validate().is_ok());
}

/// M-2: `day_equivalent_minutes` は DB の
/// `CHECK (day_equivalent_minutes BETWEEN 1 AND 1440)` と同じ範囲に揃える。
#[test]
fn adjust_request_rejects_day_equivalent_minutes_outside_db_check_range() {
    let base = LeaveLedgerAdjustRequest {
        user_id: "user-1".to_string(),
        amount_minutes: 2400,
        lot_id: None,
        day_equivalent_minutes: Some(480),
        granted_at: Some(date(2026, 7, 1)),
        expires_at: Some(date(2028, 7, 1)),
        grant_base_date: Some(date(2026, 7, 1)),
        reason: "initial migration".to_string(),
        dry_run: true,
    };
    assert!(base.validate().is_ok());

    let zero = LeaveLedgerAdjustRequest {
        day_equivalent_minutes: Some(0),
        ..base.clone()
    };
    assert!(zero.validate().is_err());

    let too_high = LeaveLedgerAdjustRequest {
        day_equivalent_minutes: Some(1441),
        ..base.clone()
    };
    assert!(too_high.validate().is_err());

    let overflow = LeaveLedgerAdjustRequest {
        day_equivalent_minutes: Some(i64::MAX),
        ..base
    };
    assert!(overflow.validate().is_err());
}

#[test]
fn adjust_response_roundtrips() {
    let response = LeaveLedgerAdjustResponse {
        dry_run: false,
        entry: Some(LeaveLedgerEntryResponse {
            id: "entry-1".to_string(),
            user_id: "user-1".to_string(),
            leave_type: "annual".to_string(),
            kind: LeaveLedgerKind::Adjust,
            lot_id: "lot-1".to_string(),
            amount_minutes: 2400,
            day_equivalent_minutes: 480,
            granted_at: Some(date(2025, 10, 1)),
            expires_at: Some(date(2027, 10, 1)),
            grant_base_date: Some(date(2025, 10, 1)),
            leave_request_id: None,
            reason: Some("initial migration".to_string()),
            created_by: Some("admin-1".to_string()),
            effective_at: Utc
                .with_ymd_and_hms(2026, 7, 5, 0, 0, 0)
                .single()
                .expect("ts"),
            created_at: Utc
                .with_ymd_and_hms(2026, 7, 5, 0, 0, 0)
                .single()
                .expect("ts"),
        }),
        balance_after_minutes: 2400,
        balance_after_days: 5.0,
    };
    let json = serde_json::to_value(&response).expect("serialize");
    let back: LeaveLedgerAdjustResponse = serde_json::from_value(json).expect("deserialize");
    assert_eq!(back, response);
}

#[test]
fn hire_date_request_and_response_roundtrip() {
    let request: SetHireDateRequest =
        serde_json::from_value(serde_json::json!({ "hire_date": "2026-04-01" }))
            .expect("deserialize");
    assert_eq!(request.hire_date, date(2026, 4, 1));

    let response = HireDateResponse {
        user_id: "user-1".to_string(),
        hire_date: date(2026, 4, 1),
    };
    let json = serde_json::to_value(&response).expect("serialize");
    let back: HireDateResponse = serde_json::from_value(json).expect("deserialize");
    assert_eq!(back, response);
}
