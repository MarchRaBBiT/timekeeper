use chrono::{TimeZone, Utc};
use timekeeper_contract::subject_requests::{
    CreateDataSubjectRequest, DataSubjectRequestResponse, DataSubjectRequestType,
    SubjectRequestDecisionPayload, SubjectRequestListResponse,
};

#[test]
fn subject_request_create_payload_serializes_snake_case_type() {
    let payload = CreateDataSubjectRequest {
        request_type: DataSubjectRequestType::Delete,
        details: Some("erase all records".to_string()),
    };

    let json = serde_json::to_value(payload).expect("serialize subject request payload");

    assert_eq!(json["request_type"], "delete");
    assert_eq!(json["details"], "erase all records");
}

#[test]
fn subject_request_create_payload_defaults_missing_details_to_none() {
    let payload: CreateDataSubjectRequest = serde_json::from_value(serde_json::json!({
        "request_type": "access"
    }))
    .expect("deserialize payload");

    assert_eq!(payload.request_type, DataSubjectRequestType::Access);
    assert!(payload.details.is_none());
}

#[test]
fn subject_request_response_deserializes_current_wire_format() {
    let response: DataSubjectRequestResponse = serde_json::from_value(serde_json::json!({
        "id": "subject-1",
        "user_id": "user-1",
        "request_type": "rectify",
        "status": "pending",
        "details": null,
        "approved_by": null,
        "approved_at": null,
        "rejected_by": null,
        "rejected_at": null,
        "cancelled_at": null,
        "decision_comment": null,
        "created_at": "2026-06-13T00:00:00Z",
        "updated_at": "2026-06-13T01:00:00Z"
    }))
    .expect("deserialize subject request response");

    assert_eq!(response.id, "subject-1");
    assert_eq!(response.user_id, "user-1");
    assert_eq!(response.request_type, DataSubjectRequestType::Rectify);
    assert_eq!(response.status, "pending");
    assert_eq!(
        response.created_at,
        Utc.with_ymd_and_hms(2026, 6, 13, 0, 0, 0)
            .single()
            .expect("created at")
    );
}

#[test]
fn subject_request_list_response_preserves_pagination_shape() {
    let response = SubjectRequestListResponse {
        page: 2,
        per_page: 25,
        total: 51,
        items: vec![DataSubjectRequestResponse {
            id: "subject-1".to_string(),
            user_id: "user-1".to_string(),
            request_type: DataSubjectRequestType::Stop,
            status: "approved".to_string(),
            details: Some("stop processing".to_string()),
            approved_by: Some("admin-1".to_string()),
            approved_at: Some(
                Utc.with_ymd_and_hms(2026, 6, 13, 2, 0, 0)
                    .single()
                    .expect("approved at"),
            ),
            rejected_by: None,
            rejected_at: None,
            cancelled_at: None,
            decision_comment: Some("ok".to_string()),
            created_at: Utc
                .with_ymd_and_hms(2026, 6, 13, 0, 0, 0)
                .single()
                .expect("created at"),
            updated_at: Utc
                .with_ymd_and_hms(2026, 6, 13, 2, 0, 0)
                .single()
                .expect("updated at"),
        }],
    };

    let json = serde_json::to_value(response).expect("serialize list response");

    assert_eq!(json["page"], 2);
    assert_eq!(json["per_page"], 25);
    assert_eq!(json["total"], 51);
    assert_eq!(json["items"][0]["request_type"], "stop");
    assert_eq!(json["items"][0]["status"], "approved");
}

#[test]
fn subject_request_decision_payload_preserves_comment() {
    let payload = SubjectRequestDecisionPayload {
        comment: "approved".to_string(),
    };

    let json = serde_json::to_value(payload).expect("serialize decision payload");

    assert_eq!(json, serde_json::json!({ "comment": "approved" }));
}
