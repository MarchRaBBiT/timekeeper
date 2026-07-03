use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use timekeeper_contract::work_schedules::{
    CoreTimeWindowInput, CoreTimeWindowResponse, CreateWorkScheduleVersionRequest, FlexPolicyInput,
    PublicHolidayPolicy, ReplaceWorkScheduleVersionRequest, ResolvedDayKind,
    ResolvedWorkdayResponse, SettlementPeriodInput, SettlementPeriodUnit, WorkScheduleSource,
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

#[test]
fn create_version_request_defaults_schedule_type_to_fixed_without_flex_fields() {
    let request: CreateWorkScheduleVersionRequest = serde_json::from_value(serde_json::json!({
        "effective_from": "2026-07-01",
        "effective_until": null,
        "timezone": "Asia/Tokyo",
        "workday_boundary": "05:00:00",
        "public_holiday_policy": "non_working",
        "late_grace_minutes": 0,
        "early_leave_grace_minutes": 0,
        "days": []
    }))
    .expect("deserialize request without schedule_type/flex_policy");

    assert_eq!(request.schedule_type, WorkScheduleType::Fixed);
    assert!(request.flex_policy.is_none());
}

#[test]
fn create_version_request_roundtrips_flex_schedule_type_and_policy() {
    let request = CreateWorkScheduleVersionRequest {
        effective_from: NaiveDate::from_ymd_opt(2026, 7, 1).expect("valid date"),
        effective_until: None,
        timezone: "Asia/Tokyo".to_string(),
        workday_boundary: time(5, 0),
        public_holiday_policy: PublicHolidayPolicy::NonWorking,
        late_grace_minutes: 0,
        early_leave_grace_minutes: 0,
        schedule_type: WorkScheduleType::Flex,
        flex_policy: Some(FlexPolicyInput {
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
        }),
        days: vec![],
    };

    let json = serde_json::to_value(&request).expect("serialize");
    assert_eq!(json["schedule_type"], "flex");
    assert_eq!(json["flex_policy"]["settlement_period"]["unit"], "monthly");

    let roundtripped: CreateWorkScheduleVersionRequest =
        serde_json::from_value(json).expect("deserialize");
    assert_eq!(roundtripped, request);
}

#[test]
fn replace_version_request_requires_explicit_schedule_type() {
    let result: Result<ReplaceWorkScheduleVersionRequest, _> =
        serde_json::from_value(serde_json::json!({
            "revision": 1,
            "effective_from": "2026-07-01",
            "effective_until": null,
            "timezone": "Asia/Tokyo",
            "workday_boundary": "05:00:00",
            "public_holiday_policy": "non_working",
            "late_grace_minutes": 0,
            "early_leave_grace_minutes": 0,
            "days": []
        }));

    assert!(
        result.is_err(),
        "replace must reject a payload without schedule_type rather than silently \
         downgrading an existing flex version to fixed"
    );
}

fn sample_resolved_workday(
    schedule_type: WorkScheduleType,
    core_time_windows: Vec<CoreTimeWindowResponse>,
) -> ResolvedWorkdayResponse {
    ResolvedWorkdayResponse {
        id: "resolved-1".to_string(),
        user_id: "user-1".to_string(),
        work_date: NaiveDate::from_ymd_opt(2026, 7, 6).expect("valid date"),
        work_schedule_id: "schedule-1".to_string(),
        work_schedule_version_id: "version-1".to_string(),
        source: WorkScheduleSource::Organization,
        day_kind: ResolvedDayKind::ScheduledWorkday,
        timezone: "Asia/Tokyo".to_string(),
        workday_boundary: time(5, 0),
        expected_work_minutes: 480,
        work_intervals: vec![],
        planned_breaks: vec![],
        schedule_type,
        core_time_windows,
        resolved_at: DateTime::<Utc>::from_timestamp(1_783_296_000, 0).expect("valid timestamp"),
        locked_at: None,
    }
}

#[test]
fn resolved_workday_response_defaults_to_fixed_with_empty_core_time_windows() {
    let response = sample_resolved_workday(WorkScheduleType::Fixed, vec![]);

    let json = serde_json::to_value(&response).expect("serialize");
    assert_eq!(json["schedule_type"], "fixed");
    assert_eq!(json["core_time_windows"], serde_json::json!([]));

    let roundtripped: ResolvedWorkdayResponse = serde_json::from_value(json).expect("deserialize");
    assert_eq!(roundtripped, response);
}

#[test]
fn resolved_workday_response_roundtrips_flex_schedule_type_and_core_time_windows() {
    let response = sample_resolved_workday(
        WorkScheduleType::Flex,
        vec![CoreTimeWindowResponse {
            weekday: 1,
            start_time: time(10, 0),
            start_day_offset: 0,
            end_time: time(15, 0),
            end_day_offset: 0,
        }],
    );

    let json = serde_json::to_value(&response).expect("serialize");
    assert_eq!(json["schedule_type"], "flex");
    assert_eq!(json["core_time_windows"][0]["weekday"], 1);
    assert_eq!(json["core_time_windows"][0]["start_time"], "10:00:00");

    let roundtripped: ResolvedWorkdayResponse = serde_json::from_value(json).expect("deserialize");
    assert_eq!(roundtripped, response);
}
