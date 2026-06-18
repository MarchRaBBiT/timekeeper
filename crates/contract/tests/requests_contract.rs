use chrono::NaiveDate;
use timekeeper_contract::requests::{
    CreateLeaveRequest, CreateOvertimeRequest, LeaveRequestResponse, OvertimeRequestResponse,
    UpdateLeaveRequest, UpdateOvertimeRequest,
};

#[test]
fn leave_request_payloads_serialize_current_wire_format() {
    let create = CreateLeaveRequest {
        leave_type: "annual".to_string(),
        start_date: NaiveDate::from_ymd_opt(2026, 6, 15).expect("start date"),
        end_date: NaiveDate::from_ymd_opt(2026, 6, 16).expect("end date"),
        reason: Some("family event".to_string()),
    };
    let update = UpdateLeaveRequest {
        leave_type: "sick".to_string(),
        start_date: NaiveDate::from_ymd_opt(2026, 6, 17).expect("start date"),
        end_date: NaiveDate::from_ymd_opt(2026, 6, 17).expect("end date"),
        reason: None,
    };

    let create_json = serde_json::to_value(create).expect("serialize create leave");
    let update_json = serde_json::to_value(update).expect("serialize update leave");

    assert_eq!(create_json["leave_type"], "annual");
    assert_eq!(create_json["start_date"], "2026-06-15");
    assert_eq!(create_json["end_date"], "2026-06-16");
    assert_eq!(create_json["reason"], "family event");
    assert_eq!(update_json["leave_type"], "sick");
    assert_eq!(update_json["start_date"], "2026-06-17");
    assert_eq!(update_json["end_date"], "2026-06-17");
    assert!(update_json["reason"].is_null());
}

#[test]
fn leave_request_response_deserializes_current_wire_format() {
    let response: LeaveRequestResponse = serde_json::from_value(serde_json::json!({
        "id": "leave-1",
        "user_id": "user-1",
        "leave_type": "annual",
        "start_date": "2026-06-15",
        "end_date": "2026-06-16",
        "reason": null,
        "status": "approved",
        "approved_by": "manager-1",
        "approved_at": "2026-06-14T01:02:03Z",
        "rejected_by": null,
        "rejected_at": null,
        "cancelled_at": null,
        "decision_comment": "ok",
        "created_at": "2026-06-13T00:00:00Z"
    }))
    .expect("deserialize leave response");

    assert_eq!(response.id, "leave-1");
    assert_eq!(response.user_id, "user-1");
    assert_eq!(response.leave_type, "annual");
    assert_eq!(response.status, "approved");
    assert_eq!(response.approved_by.as_deref(), Some("manager-1"));
    assert_eq!(
        response.approved_at.as_deref(),
        Some("2026-06-14T01:02:03Z")
    );
    assert_eq!(response.created_at, "2026-06-13T00:00:00Z");
}

#[test]
fn overtime_request_payloads_serialize_current_wire_format() {
    let create = CreateOvertimeRequest {
        date: NaiveDate::from_ymd_opt(2026, 6, 18).expect("date"),
        planned_hours: 2.5,
        reason: Some("release support".to_string()),
    };
    let update = UpdateOvertimeRequest {
        date: NaiveDate::from_ymd_opt(2026, 6, 19).expect("date"),
        planned_hours: 1.5,
        reason: None,
    };

    let create_json = serde_json::to_value(create).expect("serialize create overtime");
    let update_json = serde_json::to_value(update).expect("serialize update overtime");

    assert_eq!(create_json["date"], "2026-06-18");
    assert_eq!(create_json["planned_hours"], 2.5);
    assert_eq!(create_json["reason"], "release support");
    assert_eq!(update_json["date"], "2026-06-19");
    assert_eq!(update_json["planned_hours"], 1.5);
    assert!(update_json["reason"].is_null());
}

#[test]
fn overtime_request_response_deserializes_current_wire_format() {
    let response: OvertimeRequestResponse = serde_json::from_value(serde_json::json!({
        "id": "overtime-1",
        "user_id": "user-1",
        "date": "2026-06-18",
        "planned_hours": 2.5,
        "reason": "release support",
        "status": "pending",
        "approved_by": null,
        "approved_at": null,
        "rejected_by": null,
        "rejected_at": null,
        "cancelled_at": null,
        "decision_comment": null,
        "created_at": "2026-06-13T00:00:00Z"
    }))
    .expect("deserialize overtime response");

    assert_eq!(response.id, "overtime-1");
    assert_eq!(response.user_id, "user-1");
    assert_eq!(
        response.date,
        NaiveDate::from_ymd_opt(2026, 6, 18).expect("date")
    );
    assert_eq!(response.planned_hours, 2.5);
    assert_eq!(response.status, "pending");
    assert_eq!(response.created_at, "2026-06-13T00:00:00Z");
}
