use chrono::NaiveTime;
use timekeeper_contract::work_schedules::{
    CoreTimeWindowInput, FlexPolicyInput, SettlementPeriodInput, SettlementPeriodUnit,
    WorkScheduleType,
};
use validator::Validate;

fn time(hour: u32, minute: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(hour, minute, 0).expect("valid time")
}

#[test]
fn schedule_type_uses_snake_case_wire_format() {
    let json = serde_json::to_value(WorkScheduleType::Flex).expect("serialize");
    assert_eq!(json, "flex");

    let parsed: WorkScheduleType =
        serde_json::from_value(serde_json::json!("fixed")).expect("deserialize fixed");
    assert_eq!(parsed, WorkScheduleType::Fixed);
}

#[test]
fn settlement_period_unit_uses_snake_case_wire_format() {
    let json = serde_json::to_value(SettlementPeriodUnit::Monthly).expect("serialize");
    assert_eq!(json, "monthly");
}

#[test]
fn settlement_period_input_rejects_non_positive_minutes() {
    let request = SettlementPeriodInput {
        unit: SettlementPeriodUnit::Monthly,
        contracted_minutes_per_period: 0,
    };

    assert!(request.validate().is_err());
}

#[test]
fn flex_policy_roundtrips_core_time_windows() {
    let policy = FlexPolicyInput {
        settlement_period: SettlementPeriodInput {
            unit: SettlementPeriodUnit::Monthly,
            contracted_minutes_per_period: 9600,
        },
        core_time_windows: vec![CoreTimeWindowInput {
            weekday: 1,
            start_time: time(10, 0),
            start_day_offset: 0,
            end_time: time(15, 0),
            end_day_offset: 0,
        }],
    };

    let json = serde_json::to_value(&policy).expect("serialize");
    assert_eq!(json["settlement_period"]["unit"], "monthly");
    assert_eq!(
        json["settlement_period"]["contracted_minutes_per_period"],
        9600
    );
    assert_eq!(json["core_time_windows"][0]["weekday"], 1);
    assert_eq!(json["core_time_windows"][0]["start_time"], "10:00:00");

    let roundtripped: FlexPolicyInput = serde_json::from_value(json).expect("deserialize");
    assert_eq!(roundtripped, policy);
}

#[test]
fn flex_policy_input_defaults_core_time_windows_to_empty() {
    let policy: FlexPolicyInput = serde_json::from_value(serde_json::json!({
        "settlement_period": {
            "unit": "monthly",
            "contracted_minutes_per_period": 9600
        }
    }))
    .expect("deserialize without core_time_windows");

    assert!(policy.core_time_windows.is_empty());
}
