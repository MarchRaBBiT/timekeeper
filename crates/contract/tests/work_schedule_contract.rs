use chrono::{NaiveDate, NaiveTime};
use timekeeper_contract::work_schedules::{
    AssignmentTarget, CreateWorkScheduleRequest, CreateWorkScheduleVersionRequest, DayKind,
    PlannedBreakInput, PlannedWorkIntervalInput, PublicHolidayPolicy, WeekdayRuleInput,
    WorkScheduleAssignmentRequest,
};

#[test]
fn create_master_uses_snake_case_wire_format() {
    let request: CreateWorkScheduleRequest = serde_json::from_value(serde_json::json!({
        "code": "standard",
        "name": "Standard schedule",
        "description": null
    }))
    .expect("deserialize request");

    assert_eq!(request.code, "standard");
    assert_eq!(request.name, "Standard schedule");
    assert!(request.description.is_none());
}

#[test]
fn version_payload_roundtrips_night_shift_offsets() {
    let request = CreateWorkScheduleVersionRequest {
        effective_from: NaiveDate::from_ymd_opt(2026, 7, 1).expect("date"),
        effective_until: None,
        timezone: "Asia/Tokyo".to_string(),
        workday_boundary: NaiveTime::from_hms_opt(5, 0, 0).expect("time"),
        public_holiday_policy: PublicHolidayPolicy::NonWorking,
        late_grace_minutes: 0,
        early_leave_grace_minutes: 0,
        days: vec![WeekdayRuleInput {
            weekday: 1,
            day_kind: DayKind::WorkingDay,
            work_intervals: vec![PlannedWorkIntervalInput {
                start_time: NaiveTime::from_hms_opt(22, 0, 0).expect("time"),
                start_day_offset: 0,
                end_time: NaiveTime::from_hms_opt(7, 0, 0).expect("time"),
                end_day_offset: 1,
            }],
            planned_breaks: vec![PlannedBreakInput {
                start_time: NaiveTime::from_hms_opt(2, 0, 0).expect("time"),
                start_day_offset: 1,
                end_time: NaiveTime::from_hms_opt(3, 0, 0).expect("time"),
                end_day_offset: 1,
            }],
        }],
    };

    let json = serde_json::to_value(&request).expect("serialize");
    assert_eq!(json["public_holiday_policy"], "non_working");
    assert_eq!(json["days"][0]["day_kind"], "working_day");
    assert_eq!(json["days"][0]["work_intervals"][0]["end_day_offset"], 1);
}

#[test]
fn assignment_target_is_explicitly_tagged() {
    let request = WorkScheduleAssignmentRequest {
        work_schedule_id: "018f0000-0000-7000-8000-000000000001".to_string(),
        target: AssignmentTarget::Department {
            department_id: "department-1".to_string(),
        },
        valid_from: NaiveDate::from_ymd_opt(2026, 7, 1).expect("date"),
        valid_until: None,
    };

    let json = serde_json::to_value(request).expect("serialize");
    assert_eq!(json["target"]["type"], "department");
    assert_eq!(json["target"]["department_id"], "department-1");
}
