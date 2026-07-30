use chrono::{NaiveDate, TimeZone, Utc};
use timekeeper_contract::attendance::{
    AttendanceCorrectionDecisionPayload, AttendanceCorrectionResponse,
    AttendanceCorrectionSnapshot, AttendanceCorrectionStatus, AttendanceLeaveResponse,
    AttendanceResponse, AttendanceStatusResponse, AttendanceSummary, BreakEndRequest,
    BreakRecordResponse, BreakStartRequest, ClassificationTotalsResponse, ClockInRequest,
    ClockOutRequest, CorrectionBreakItem, CreateAttendanceCorrectionRequest,
    DailyClassificationResponse, FlexPeriodClassificationResponse, FlexPeriodStatusResponse,
    MonthlyClassificationResponse, UpdateAttendanceCorrectionRequest,
};
use timekeeper_contract::work_schedules::{ResolvedDayKind, WorkScheduleType};

#[test]
fn clock_in_request_omits_date_when_client_uses_current_day() {
    let request = ClockInRequest { date: None };

    let json = serde_json::to_value(request).expect("serialize request");

    assert_eq!(json, serde_json::json!({}));
}

#[test]
fn clock_in_request_serializes_explicit_work_date() {
    let request = ClockInRequest {
        date: Some(NaiveDate::from_ymd_opt(2026, 6, 12).expect("date")),
    };

    let json = serde_json::to_value(request).expect("serialize request");

    assert_eq!(json, serde_json::json!({ "date": "2026-06-12" }));
}

#[test]
fn clock_out_request_uses_same_optional_date_wire_format() {
    let request = ClockOutRequest { date: None };

    let json = serde_json::to_value(request).expect("serialize request");

    assert_eq!(json, serde_json::json!({}));
}

#[test]
fn break_start_request_keeps_attendance_id_key() {
    let request = BreakStartRequest {
        attendance_id: "attendance-1".to_string(),
    };

    let json = serde_json::to_value(request).expect("serialize request");

    assert_eq!(json, serde_json::json!({ "attendance_id": "attendance-1" }));
}

#[test]
fn break_end_request_keeps_break_record_id_key() {
    let request = BreakEndRequest {
        break_record_id: "break-1".to_string(),
    };

    let json = serde_json::to_value(request).expect("serialize request");

    assert_eq!(json, serde_json::json!({ "break_record_id": "break-1" }));
}

#[test]
fn attendance_status_response_deserializes_current_wire_format() {
    let response: AttendanceStatusResponse = serde_json::from_value(serde_json::json!({
        "status": "on_break",
        "attendance_id": "attendance-1",
        "active_break_id": "break-1",
        "clock_in_time": "2026-06-12T09:00:00",
        "clock_out_time": null
    }))
    .expect("deserialize status response");

    assert_eq!(response.status, "on_break");
    assert_eq!(response.attendance_id.as_deref(), Some("attendance-1"));
    assert_eq!(response.active_break_id.as_deref(), Some("break-1"));
    assert!(response.clock_in_time.is_some());
    assert!(response.clock_out_time.is_none());
}

#[test]
fn break_record_response_deserializes_current_wire_format() {
    let response: BreakRecordResponse = serde_json::from_value(serde_json::json!({
        "id": "break-1",
        "attendance_id": "attendance-1",
        "break_start_time": "2026-06-12T12:00:00",
        "break_end_time": "2026-06-12T12:45:00",
        "duration_minutes": 45
    }))
    .expect("deserialize break record response");

    assert_eq!(response.id, "break-1");
    assert_eq!(response.attendance_id, "attendance-1");
    assert!(response
        .break_start_time
        .to_string()
        .starts_with("2026-06-12 12:00:00"));
    assert_eq!(response.duration_minutes, Some(45));
}

#[test]
fn attendance_summary_serializes_current_wire_format() {
    let summary = AttendanceSummary {
        month: 6,
        year: 2026,
        total_work_hours: 160.5,
        total_work_days: 20,
        average_daily_hours: 8.025,
        leave_days: 2,
    };

    let json = serde_json::to_value(summary).expect("serialize summary");

    assert_eq!(
        json,
        serde_json::json!({
            "month": 6,
            "year": 2026,
            "total_work_hours": 160.5,
            "total_work_days": 20,
            "average_daily_hours": 8.025,
            "leave_days": 2
        })
    );
}

#[test]
fn attendance_summary_deserializes_legacy_payload_without_leave_days() {
    let summary: AttendanceSummary = serde_json::from_value(serde_json::json!({
        "month": 6,
        "year": 2026,
        "total_work_hours": 160.5,
        "total_work_days": 20,
        "average_daily_hours": 8.025
    }))
    .expect("deserialize legacy summary");

    assert_eq!(summary.leave_days, 0);
}

#[test]
fn attendance_response_serializes_current_wire_format() {
    let date = NaiveDate::from_ymd_opt(2026, 6, 12).expect("date");
    let response = AttendanceResponse {
        id: "attendance-1".to_string(),
        user_id: "user-1".to_string(),
        date,
        clock_in_time: Some(date.and_hms_opt(9, 0, 0).expect("clock in")),
        clock_out_time: Some(date.and_hms_opt(18, 0, 0).expect("clock out")),
        status: "present".to_string(),
        total_work_hours: Some(8.0),
        break_records: vec![BreakRecordResponse {
            id: "break-1".to_string(),
            attendance_id: "attendance-1".to_string(),
            break_start_time: date.and_hms_opt(12, 0, 0).expect("break start"),
            break_end_time: Some(date.and_hms_opt(13, 0, 0).expect("break end")),
            duration_minutes: Some(60),
        }],
        leave: None,
    };

    let json = serde_json::to_value(response).expect("serialize response");

    assert_eq!(json["id"], "attendance-1");
    assert_eq!(json["user_id"], "user-1");
    assert_eq!(json["date"], "2026-06-12");
    assert_eq!(json["status"], "present");
    assert_eq!(json["break_records"][0]["id"], "break-1");
    assert_eq!(json["break_records"][0]["attendance_id"], "attendance-1");
    assert!(
        json.get("leave").is_none(),
        "leave must be omitted when absent to keep the legacy wire format"
    );
}

#[test]
fn attendance_response_roundtrips_leave_designation() {
    let date = NaiveDate::from_ymd_opt(2026, 7, 6).expect("date");
    let response = AttendanceResponse {
        id: "leave:request-1:2026-07-06".to_string(),
        user_id: "user-1".to_string(),
        date,
        clock_in_time: None,
        clock_out_time: None,
        status: "on_leave".to_string(),
        total_work_hours: None,
        break_records: Vec::new(),
        leave: Some(AttendanceLeaveResponse {
            leave_request_id: "request-1".to_string(),
            leave_type: "annual".to_string(),
            acquisition_unit: timekeeper_contract::requests::LeaveAcquisitionUnit::Day,
            start_time: None,
            end_time: None,
            requested_minutes: None,
        }),
    };

    let json = serde_json::to_value(&response).expect("serialize response");
    assert_eq!(json["status"], "on_leave");
    assert_eq!(json["leave"]["leave_request_id"], "request-1");
    assert_eq!(json["leave"]["leave_type"], "annual");

    let roundtrip: AttendanceResponse = serde_json::from_value(json).expect("deserialize response");
    assert_eq!(roundtrip, response);
}

#[test]
fn attendance_response_deserializes_legacy_payload_without_leave() {
    let response: AttendanceResponse = serde_json::from_value(serde_json::json!({
        "id": "attendance-1",
        "user_id": "user-1",
        "date": "2026-06-12",
        "clock_in_time": "2026-06-12T09:00:00",
        "clock_out_time": "2026-06-12T18:00:00",
        "status": "present",
        "total_work_hours": 8.0,
        "break_records": []
    }))
    .expect("deserialize legacy response");

    assert!(response.leave.is_none());
}

#[test]
fn admin_attendance_upsert_serializes_current_wire_format() {
    let request = timekeeper_contract::attendance::AdminAttendanceUpsert {
        user_id: "user-1".to_string(),
        date: "2026-06-12".to_string(),
        clock_in_time: "2026-06-12T09:00:00".to_string(),
        clock_out_time: Some("2026-06-12T18:00:00".to_string()),
        breaks: Some(vec![timekeeper_contract::attendance::AdminBreakItem {
            break_start_time: "2026-06-12T12:00:00".to_string(),
            break_end_time: Some("2026-06-12T13:00:00".to_string()),
        }]),
    };

    let json = serde_json::to_value(request).expect("serialize admin upsert request");

    assert_eq!(json["user_id"], "user-1");
    assert_eq!(json["date"], "2026-06-12");
    assert_eq!(json["clock_in_time"], "2026-06-12T09:00:00");
    assert_eq!(json["clock_out_time"], "2026-06-12T18:00:00");
    assert_eq!(json["breaks"][0]["break_start_time"], "2026-06-12T12:00:00");
    assert_eq!(json["breaks"][0]["break_end_time"], "2026-06-12T13:00:00");
}

#[test]
fn active_break_response_deserializes_current_wire_format() {
    let response: timekeeper_contract::attendance::ActiveBreakResponse =
        serde_json::from_value(serde_json::json!({
            "break_id": "break-1",
            "attendance_id": "attendance-1",
            "user_id": "user-1",
            "username": "alice",
            "full_name": "Alice Example",
            "break_start_time": "2026-06-12T12:00:00"
        }))
        .expect("deserialize active break response");

    assert_eq!(response.break_id, "break-1");
    assert_eq!(response.attendance_id, "attendance-1");
    assert_eq!(response.user_id, "user-1");
    assert_eq!(response.username, "alice");
    assert_eq!(response.full_name.as_deref(), Some("Alice Example"));
    assert!(response
        .break_start_time
        .to_string()
        .starts_with("2026-06-12 12:00:00"));
}

#[test]
fn attendance_correction_request_serializes_current_wire_format() {
    let date = NaiveDate::from_ymd_opt(2026, 6, 12).expect("date");
    let request = CreateAttendanceCorrectionRequest {
        date,
        clock_in_time: Some(date.and_hms_opt(9, 0, 0).expect("clock in")),
        clock_out_time: Some(date.and_hms_opt(18, 0, 0).expect("clock out")),
        breaks: Some(vec![CorrectionBreakItem {
            break_start_time: date.and_hms_opt(12, 0, 0).expect("break start"),
            break_end_time: Some(date.and_hms_opt(13, 0, 0).expect("break end")),
        }]),
        reason: "Forgot to end break".to_string(),
    };

    let json = serde_json::to_value(request).expect("serialize correction request");

    assert_eq!(json["date"], "2026-06-12");
    assert_eq!(json["clock_in_time"], "2026-06-12T09:00:00");
    assert_eq!(json["clock_out_time"], "2026-06-12T18:00:00");
    assert_eq!(json["breaks"][0]["break_start_time"], "2026-06-12T12:00:00");
    assert_eq!(json["breaks"][0]["break_end_time"], "2026-06-12T13:00:00");
    assert_eq!(json["reason"], "Forgot to end break");
}

#[test]
fn attendance_correction_update_request_preserves_optional_breaks_shape() {
    let date = NaiveDate::from_ymd_opt(2026, 6, 12).expect("date");
    let request = UpdateAttendanceCorrectionRequest {
        clock_in_time: None,
        clock_out_time: Some(date.and_hms_opt(19, 0, 0).expect("clock out")),
        breaks: None,
        reason: "Worked late".to_string(),
    };

    let json = serde_json::to_value(request).expect("serialize update correction request");

    assert!(json["clock_in_time"].is_null());
    assert_eq!(json["clock_out_time"], "2026-06-12T19:00:00");
    assert!(json["breaks"].is_null());
    assert_eq!(json["reason"], "Worked late");
}

#[test]
fn attendance_correction_response_deserializes_current_wire_format() {
    let response: AttendanceCorrectionResponse = serde_json::from_value(serde_json::json!({
        "id": "request-1",
        "user_id": "user-1",
        "attendance_id": "attendance-1",
        "date": "2026-06-12",
        "status": "approved",
        "reason": "Forgot to clock out",
        "original_snapshot": {
            "clock_in_time": "2026-06-12T09:00:00",
            "clock_out_time": null,
            "breaks": []
        },
        "proposed_values": {
            "clock_in_time": "2026-06-12T09:00:00",
            "clock_out_time": "2026-06-12T18:00:00",
            "breaks": [
                {
                    "break_start_time": "2026-06-12T12:00:00",
                    "break_end_time": "2026-06-12T13:00:00"
                }
            ]
        },
        "decision_comment": "Looks correct",
        "approved_by": "manager-1",
        "approved_at": "2026-06-13T01:00:00Z",
        "rejected_by": null,
        "rejected_at": null,
        "cancelled_at": null,
        "created_at": "2026-06-12T10:00:00Z",
        "updated_at": "2026-06-13T01:00:00Z"
    }))
    .expect("deserialize correction response");

    assert_eq!(response.status, AttendanceCorrectionStatus::Approved);
    assert_eq!(response.user_id, "user-1");
    assert_eq!(response.attendance_id, "attendance-1");
    assert_eq!(response.proposed_values.breaks.len(), 1);
    assert_eq!(response.approved_by.as_deref(), Some("manager-1"));
    assert!(response.rejected_at.is_none());
}

#[test]
fn attendance_correction_response_serializes_snake_case_status() {
    let date = NaiveDate::from_ymd_opt(2026, 6, 12).expect("date");
    let response = AttendanceCorrectionResponse {
        id: "request-1".to_string(),
        user_id: "user-1".to_string(),
        attendance_id: "attendance-1".to_string(),
        date,
        status: AttendanceCorrectionStatus::Cancelled,
        reason: "No longer needed".to_string(),
        original_snapshot: AttendanceCorrectionSnapshot {
            clock_in_time: Some(date.and_hms_opt(9, 0, 0).expect("clock in")),
            clock_out_time: None,
            breaks: Vec::new(),
        },
        proposed_values: AttendanceCorrectionSnapshot {
            clock_in_time: Some(date.and_hms_opt(9, 0, 0).expect("clock in")),
            clock_out_time: Some(date.and_hms_opt(18, 0, 0).expect("clock out")),
            breaks: Vec::new(),
        },
        decision_comment: None,
        approved_by: None,
        approved_at: None,
        rejected_by: None,
        rejected_at: None,
        cancelled_at: Some(Utc.with_ymd_and_hms(2026, 6, 12, 11, 0, 0).unwrap()),
        created_at: Utc.with_ymd_and_hms(2026, 6, 12, 10, 0, 0).unwrap(),
        updated_at: Utc.with_ymd_and_hms(2026, 6, 12, 11, 0, 0).unwrap(),
    };

    let json = serde_json::to_value(response).expect("serialize correction response");

    assert_eq!(json["status"], "cancelled");
    assert_eq!(json["cancelled_at"], "2026-06-12T11:00:00Z");
}

#[test]
fn attendance_correction_decision_payload_keeps_comment_field() {
    let payload = AttendanceCorrectionDecisionPayload {
        comment: "Approved".to_string(),
    };

    let json = serde_json::to_value(payload).expect("serialize decision payload");

    assert_eq!(json, serde_json::json!({ "comment": "Approved" }));
}

#[test]
fn monthly_classification_calculated_round_trips_tagged_status() {
    let response = MonthlyClassificationResponse::Calculated {
        year: 2026,
        month: 7,
        days: vec![DailyClassificationResponse {
            work_date: NaiveDate::from_ymd_opt(2026, 7, 6).expect("date"),
            day_kind: ResolvedDayKind::ScheduledWorkday,
            schedule_type: WorkScheduleType::Fixed,
            actual_minutes: 540,
            scheduled_minutes: 480,
            statutory_within_minutes: 0,
            statutory_excess_minutes: 60,
            legal_holiday_minutes: 0,
            night_minutes: 0,
            in_progress: false,
            locked: true,
        }],
        totals: ClassificationTotalsResponse {
            actual_minutes: 540,
            scheduled_minutes: 480,
            statutory_within_minutes: 0,
            statutory_excess_minutes: 60,
            legal_holiday_minutes: 0,
            night_minutes: 0,
        },
        flex_period: FlexPeriodStatusResponse::Calculated {
            result: FlexPeriodClassificationResponse {
                contracted_minutes: 9600,
                statutory_frame_minutes: 10628,
                actual_minutes: 540,
                scheduled_minutes: 540,
                statutory_within_minutes: 0,
                statutory_excess_minutes: 0,
            },
        },
    };

    let json = serde_json::to_value(&response).expect("serialize classification");

    assert_eq!(json["status"], "calculated");
    assert_eq!(json["days"][0]["day_kind"], "scheduled_workday");
    assert_eq!(json["days"][0]["schedule_type"], "fixed");
    assert_eq!(json["flex_period"]["status"], "calculated");
    assert_eq!(json["flex_period"]["contracted_minutes"], 9600);
    let round_trip: MonthlyClassificationResponse =
        serde_json::from_value(json).expect("deserialize classification");
    assert_eq!(round_trip, response);
}

#[test]
fn monthly_classification_fail_closed_variants_are_tagged() {
    let unresolved =
        serde_json::to_value(MonthlyClassificationResponse::UnresolvedDays).expect("serialize");
    let not_configured = serde_json::to_value(MonthlyClassificationResponse::WorkRuleNotConfigured)
        .expect("serialize");

    assert_eq!(
        unresolved,
        serde_json::json!({ "status": "unresolved_days" })
    );
    assert_eq!(
        not_configured,
        serde_json::json!({ "status": "work_rule_not_configured" })
    );

    let flex_variants = [
        (
            FlexPeriodStatusResponse::NotApplicable,
            serde_json::json!({ "status": "not_applicable" }),
        ),
        (
            FlexPeriodStatusResponse::UnresolvedDays,
            serde_json::json!({ "status": "unresolved_days" }),
        ),
        (
            FlexPeriodStatusResponse::VersionMixed,
            serde_json::json!({ "status": "version_mixed" }),
        ),
        (
            FlexPeriodStatusResponse::NotConfigured,
            serde_json::json!({ "status": "not_configured" }),
        ),
    ];
    for (value, expected) in flex_variants {
        assert_eq!(serde_json::to_value(value).expect("serialize"), expected);
    }
}
