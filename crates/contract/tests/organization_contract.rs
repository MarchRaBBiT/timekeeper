use chrono::{TimeZone, Utc};
use timekeeper_contract::organization::{
    AssignManagerRequest, CreateDepartmentRequest, DepartmentResponse, UpdateDepartmentRequest,
};

#[test]
fn create_department_request_omits_missing_parent_id() {
    let request = CreateDepartmentRequest {
        name: "Engineering".to_string(),
        parent_id: None,
    };

    let json = serde_json::to_value(request).expect("serialize create department request");

    assert_eq!(json["name"], "Engineering");
    assert!(json.get("parent_id").is_none());
}

#[test]
fn update_department_request_omits_absent_fields_but_preserves_empty_parent() {
    let no_change = UpdateDepartmentRequest {
        name: None,
        parent_id: None,
    };
    let clear_parent = UpdateDepartmentRequest {
        name: Some("Platform".to_string()),
        parent_id: Some(String::new()),
    };

    let no_change_json = serde_json::to_value(no_change).expect("serialize no-change update");
    let clear_parent_json =
        serde_json::to_value(clear_parent).expect("serialize clear-parent update");

    assert_eq!(no_change_json, serde_json::json!({}));
    assert_eq!(clear_parent_json["name"], "Platform");
    assert_eq!(clear_parent_json["parent_id"], "");
}

#[test]
fn department_response_deserializes_current_wire_format() {
    let response: DepartmentResponse = serde_json::from_value(serde_json::json!({
        "id": "department-1",
        "name": "Engineering",
        "parent_id": null,
        "created_at": "2026-06-13T00:00:00Z",
        "updated_at": "2026-06-13T01:00:00Z"
    }))
    .expect("deserialize department response");

    assert_eq!(response.id, "department-1");
    assert_eq!(response.name, "Engineering");
    assert!(response.parent_id.is_none());
    assert_eq!(
        response.updated_at,
        Utc.with_ymd_and_hms(2026, 6, 13, 1, 0, 0)
            .single()
            .expect("updated at")
    );
}

#[test]
fn assign_manager_request_preserves_user_id() {
    let request = AssignManagerRequest {
        user_id: "user-1".to_string(),
    };

    let json = serde_json::to_value(request).expect("serialize assign manager request");

    assert_eq!(json, serde_json::json!({ "user_id": "user-1" }));
}
