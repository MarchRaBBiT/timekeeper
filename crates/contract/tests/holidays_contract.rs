use chrono::{NaiveDate, TimeZone, Utc};
use timekeeper_contract::holidays::{
    AdminHolidayKind, AdminHolidayListItem, AdminHolidayListResponse, CreateHolidayRequest,
    CreateWeeklyHolidayRequest, GoogleHolidayCandidate, HolidayCalendarEntry, HolidayCheckResponse,
    HolidayResponse, WeeklyHolidayResponse,
};

#[test]
fn create_holiday_request_preserves_nullable_description() {
    let request = CreateHolidayRequest {
        holiday_date: NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date"),
        name: "New Year".to_string(),
        description: None,
    };

    let json = serde_json::to_value(request).expect("serialize create holiday request");

    assert_eq!(json["holiday_date"], "2026-01-01");
    assert_eq!(json["name"], "New Year");
    assert!(json.get("description").is_some());
    assert!(json["description"].is_null());
}

#[test]
fn google_holiday_candidate_accepts_missing_description() {
    let candidate: GoogleHolidayCandidate = serde_json::from_value(serde_json::json!({
        "holiday_date": "2026-02-11",
        "name": "Foundation Day"
    }))
    .expect("deserialize google holiday candidate");

    assert_eq!(
        candidate.holiday_date,
        NaiveDate::from_ymd_opt(2026, 2, 11).expect("valid date")
    );
    assert_eq!(candidate.name, "Foundation Day");
    assert!(candidate.description.is_none());
}

#[test]
fn holiday_response_deserializes_current_wire_format() {
    let response: HolidayResponse = serde_json::from_value(serde_json::json!({
        "id": "holiday-1",
        "holiday_date": "2026-05-03",
        "name": "Constitution Memorial Day",
        "description": null
    }))
    .expect("deserialize holiday response");

    assert_eq!(response.id, "holiday-1");
    assert_eq!(
        response.holiday_date,
        NaiveDate::from_ymd_opt(2026, 5, 3).expect("valid date")
    );
    assert!(response.description.is_none());
}

#[test]
fn weekly_holiday_request_defaults_missing_end_date() {
    let request: CreateWeeklyHolidayRequest = serde_json::from_value(serde_json::json!({
        "weekday": 6,
        "starts_on": "2026-01-03"
    }))
    .expect("deserialize weekly holiday request");

    assert_eq!(request.weekday, 6);
    assert_eq!(
        request.starts_on,
        NaiveDate::from_ymd_opt(2026, 1, 3).expect("valid date")
    );
    assert!(request.ends_on.is_none());
}

#[test]
fn weekly_holiday_response_uses_existing_numeric_weekday_shape() {
    let response: WeeklyHolidayResponse = serde_json::from_value(serde_json::json!({
        "id": "weekly-1",
        "weekday": 0,
        "starts_on": "2026-01-04",
        "ends_on": null,
        "enforced_from": "2026-01-04",
        "enforced_to": null
    }))
    .expect("deserialize weekly holiday response");

    assert_eq!(response.id, "weekly-1");
    assert_eq!(response.weekday, 0);
    assert!(response.ends_on.is_none());
}

#[test]
fn admin_holiday_list_response_preserves_kind_and_optional_fields() {
    let item = AdminHolidayListItem {
        id: "holiday-1".to_string(),
        kind: AdminHolidayKind::Public,
        applies_from: NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date"),
        applies_to: None,
        date: Some(NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date")),
        weekday: None,
        starts_on: None,
        ends_on: None,
        name: Some("New Year".to_string()),
        description: None,
        user_id: None,
        reason: Some("public holiday".to_string()),
        created_by: Some("admin-1".to_string()),
        created_at: Utc
            .with_ymd_and_hms(2026, 1, 1, 0, 0, 0)
            .single()
            .expect("created at"),
        is_override: None,
    };
    let response = AdminHolidayListResponse {
        page: 1,
        per_page: 25,
        total: 1,
        items: vec![item],
    };

    let json = serde_json::to_value(response).expect("serialize admin holiday list response");

    assert_eq!(json["items"][0]["kind"], "public");
    assert_eq!(json["items"][0]["applies_from"], "2026-01-01");
    assert!(json["items"][0]["is_override"].is_null());
}

#[test]
fn public_holiday_service_responses_keep_existing_fields() {
    let check: HolidayCheckResponse = serde_json::from_value(serde_json::json!({
        "is_holiday": true,
        "reason": "weekly holiday"
    }))
    .expect("deserialize holiday check response");
    let calendar: HolidayCalendarEntry = serde_json::from_value(serde_json::json!({
        "date": "2026-01-04",
        "reason": "weekly holiday"
    }))
    .expect("deserialize holiday calendar entry");

    assert!(check.is_holiday);
    assert_eq!(check.reason.as_deref(), Some("weekly holiday"));
    assert_eq!(
        calendar.date,
        NaiveDate::from_ymd_opt(2026, 1, 4).expect("valid date")
    );
}
