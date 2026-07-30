use chrono::NaiveDate;
use timekeeper_contract::settlement_balance::{
    SettlementBalanceDayResponse, SettlementBalanceResponse,
};

#[test]
fn every_status_round_trips_with_a_tagged_status_field() {
    let calculated = SettlementBalanceResponse::Calculated {
        year: 2026,
        month: 7,
        contracted_minutes: 9600,
        actual_minutes: 9300,
        balance_minutes: -300,
        days: vec![SettlementBalanceDayResponse {
            work_date: NaiveDate::from_ymd_opt(2026, 7, 1).expect("date"),
            actual_minutes: 480,
            locked: true,
            in_progress: false,
        }],
    };
    let cases = [
        calculated,
        SettlementBalanceResponse::UnresolvedDays,
        SettlementBalanceResponse::NotApplicable,
        SettlementBalanceResponse::VersionMixed,
        SettlementBalanceResponse::NotConfigured,
    ];

    for expected in cases {
        let value = serde_json::to_value(&expected).expect("serialize");
        assert!(value["status"].is_string());
        let actual: SettlementBalanceResponse = serde_json::from_value(value).expect("deserialize");
        assert_eq!(actual, expected);
    }
}

#[test]
fn unavailable_status_has_no_misleading_calculated_fields() {
    let value =
        serde_json::to_value(SettlementBalanceResponse::VersionMixed).expect("serialize status");
    assert_eq!(value, serde_json::json!({"status": "version_mixed"}));
}
