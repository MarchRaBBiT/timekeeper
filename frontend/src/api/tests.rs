#![cfg(not(coverage))]

use super::*;
use httpmock::prelude::*;
use serde_json::json;

fn user_json(id: &str) -> serde_json::Value {
    json!({
        "id": id,
        "username": "alice",
        "full_name": "Alice Example",
        "role": "admin",
        "is_system_admin": true,
        "mfa_enabled": false
    })
}

fn archived_user_json(id: &str) -> serde_json::Value {
    json!({
        "id": id,
        "username": "archived",
        "full_name": "Archived User",
        "role": "member",
        "is_system_admin": false,
        "archived_at": "2025-01-02T10:00:00Z",
        "archived_by": "admin"
    })
}

fn attendance_json(id: &str) -> serde_json::Value {
    json!({
        "id": id,
        "user_id": "u1",
        "date": "2025-01-02",
        "clock_in_time": "2025-01-02T09:00:00",
        "clock_out_time": null,
        "status": "clocked_in",
        "total_work_hours": null,
        "break_records": []
    })
}

fn break_record_json(id: &str) -> serde_json::Value {
    json!({
        "id": id,
        "attendance_id": "att-1",
        "break_start_time": "2025-01-02T12:00:00",
        "break_end_time": null,
        "duration_minutes": null
    })
}

fn attendance_status_json(status: &str) -> serde_json::Value {
    json!({
        "status": status,
        "attendance_id": "att-1",
        "active_break_id": null,
        "clock_in_time": "2025-01-02T09:00:00",
        "clock_out_time": null
    })
}

fn attendance_summary_json() -> serde_json::Value {
    json!({
        "month": 1,
        "year": 2025,
        "total_work_hours": 160.0,
        "total_work_days": 20,
        "average_daily_hours": 8.0
    })
}

fn holiday_list_json() -> serde_json::Value {
    json!({
        "page": 1,
        "per_page": 50,
        "total": 1,
        "items": [{
            "id": "h1",
            "kind": "public",
            "applies_from": "2025-01-01",
            "applies_to": null,
            "date": "2025-01-01",
            "weekday": null,
            "starts_on": null,
            "ends_on": null,
            "name": "New Year",
            "description": "Holiday",
            "user_id": null,
            "reason": null,
            "created_by": "admin",
            "created_at": "2025-01-01T00:00:00Z",
            "is_override": null
        }]
    })
}

fn weekly_holiday_json() -> serde_json::Value {
    json!({
        "id": "wh1",
        "weekday": 1,
        "starts_on": "2025-01-01",
        "ends_on": null,
        "enforced_from": "2025-01-01",
        "enforced_to": null
    })
}

fn subject_request_json(id: &str) -> serde_json::Value {
    json!({
        "id": id,
        "user_id": "u1",
        "request_type": "access",
        "status": "pending",
        "details": null,
        "approved_by": null,
        "approved_at": null,
        "rejected_by": null,
        "rejected_at": null,
        "cancelled_at": null,
        "decision_comment": null,
        "created_at": "2025-01-01T00:00:00Z",
        "updated_at": "2025-01-01T00:00:00Z"
    })
}

fn leave_request_json(id: &str) -> serde_json::Value {
    json!({
        "id": id,
        "user_id": "u1",
        "leave_type": "annual",
        "start_date": "2025-01-10",
        "end_date": "2025-01-12",
        "reason": null,
        "status": "pending",
        "approved_by": null,
        "approved_at": null,
        "rejected_by": null,
        "rejected_at": null,
        "cancelled_at": null,
        "decision_comment": null,
        "created_at": "2025-01-01T00:00:00Z"
    })
}

fn overtime_request_json(id: &str) -> serde_json::Value {
    json!({
        "id": id,
        "user_id": "u1",
        "date": "2025-01-11",
        "planned_hours": 2.5,
        "reason": null,
        "status": "pending",
        "approved_by": null,
        "approved_at": null,
        "rejected_by": null,
        "rejected_at": null,
        "cancelled_at": null,
        "decision_comment": null,
        "created_at": "2025-01-01T00:00:00Z"
    })
}

fn api_client(server: &MockServer) -> ApiClient {
    ApiClient::new_with_base_url(&server.url("/api"))
}

#[tokio::test]
async fn api_client_admin_and_auth_endpoints_succeed() {
    let server = MockServer::start_async().await;

    server.mock(|when, then| {
        when.method(GET).path("/api/auth/me");
        then.status(200).json_body(user_json("u1"));
    });
    server.mock(|when, then| {
        when.method(GET).path("/api/admin/users");
        then.status(200).json_body(json!([user_json("u1")]));
    });
    server.mock(|when, then| {
        when.method(POST).path("/api/admin/users");
        then.status(200).json_body(user_json("u2"));
    });
    server.mock(|when, then| {
        when.method(POST).path("/api/admin/mfa/reset");
        then.status(200).json_body(json!({}));
    });
    server.mock(|when, then| {
        when.method(DELETE).path("/api/admin/users/u1");
        then.status(200).json_body(json!({}));
    });
    server.mock(|when, then| {
        when.method(GET).path("/api/admin/archived-users");
        then.status(200).json_body(json!([archived_user_json("a1")]));
    });
    server.mock(|when, then| {
        when.method(POST)
            .path("/api/admin/archived-users/a1/restore");
        then.status(200).json_body(json!({}));
    });
    server.mock(|when, then| {
        when.method(DELETE).path("/api/admin/archived-users/a1");
        then.status(200).json_body(json!({}));
    });

    server.mock(|when, then| {
        when.method(GET).path("/api/admin/holidays");
        then.status(200).json_body(holiday_list_json());
    });
    server.mock(|when, then| {
        when.method(POST).path("/api/admin/holidays");
        then.status(200).json_body(json!({
            "id": "h2",
            "holiday_date": "2025-02-01",
            "name": "Test Holiday",
            "description": null
        }));
    });
    server.mock(|when, then| {
        when.method(DELETE).path("/api/admin/holidays/h2");
        then.status(200).json_body(json!({}));
    });
    server.mock(|when, then| {
        when.method(GET).path("/api/holidays/check");
        then.status(200).json_body(json!({ "is_holiday": true, "reason": "public holiday" }));
    });
    server.mock(|when, then| {
        when.method(GET).path("/api/holidays/month");
        then.status(200).json_body(json!([{ "date": "2025-01-01", "reason": "public holiday" }]));
    });
    server.mock(|when, then| {
        when.method(GET).path("/api/admin/holidays/weekly");
        then.status(200).json_body(json!([weekly_holiday_json()]));
    });
    server.mock(|when, then| {
        when.method(POST).path("/api/admin/holidays/weekly");
        then.status(200).json_body(weekly_holiday_json());
    });
    server.mock(|when, then| {
        when.method(DELETE).path("/api/admin/holidays/weekly/wh1");
        then.status(200).json_body(json!({}));
    });
    server.mock(|when, then| {
        when.method(GET).path("/api/admin/holidays/google");
        then.status(200).json_body(json!([{
            "holiday_date": "2025-01-03",
            "name": "Imported",
            "description": null
        }]));
    });
    server.mock(|when, then| {
        when.method(GET).path("/api/admin/export");
        then.status(200).json_body(json!({ "filename": "export.csv", "csv_data": "a,b\\n1,2" }));
    });

    server.mock(|when, then| {
        when.method(POST).path("/api/auth/request-password-reset");
        then.status(200).json_body(json!({ "message": "sent" }));
    });
    server.mock(|when, then| {
        when.method(POST).path("/api/auth/reset-password");
        then.status(200).json_body(json!({ "message": "ok" }));
    });

    let client = api_client(&server);
    let me = client.get_me().await.unwrap();
    assert_eq!(me.id, "u1");
    assert_eq!(client.get_users().await.unwrap().len(), 1);
    assert_eq!(client.create_user(CreateUser {
        username: "bob".into(),
        password: "secret".into(),
        full_name: "Bob".into(),
        email: "bob@example.com".into(),
        role: "member".into(),
        is_system_admin: false,
    }).await.unwrap().id, "u2");
    client.admin_reset_mfa("u1").await.unwrap();
    client.admin_delete_user("u1", false).await.unwrap();
    assert_eq!(client.admin_get_archived_users().await.unwrap().len(), 1);
    client.admin_restore_archived_user("a1").await.unwrap();
    client.admin_delete_archived_user("a1").await.unwrap();

    let holidays = client.admin_list_holidays(1, 50, None, None).await.unwrap();
    assert_eq!(holidays.total, 1);
    let created = client.admin_create_holiday(&CreateHolidayRequest {
        holiday_date: chrono::NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        name: "Test Holiday".into(),
        description: None,
    }).await.unwrap();
    assert_eq!(created.id, "h2");
    client.admin_delete_holiday("h2").await.unwrap();
    let check = client.check_holiday(chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()).await.unwrap();
    assert!(check.is_holiday);
    assert_eq!(client.get_monthly_holidays(2025, 1).await.unwrap().len(), 1);
    assert_eq!(client.admin_list_weekly_holidays().await.unwrap().len(), 1);
    let weekly = client.admin_create_weekly_holiday(&CreateWeeklyHolidayRequest {
        weekday: 1,
        starts_on: chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        ends_on: None,
    }).await.unwrap();
    assert_eq!(weekly.id, "wh1");
    client.admin_delete_weekly_holiday("wh1").await.unwrap();
    assert_eq!(client.admin_fetch_google_holidays(Some(2025)).await.unwrap().len(), 1);

    let export = client.export_data_filtered(Some("alice"), Some("2025-01-01"), Some("2025-01-31")).await.unwrap();
    assert_eq!(export["filename"], "export.csv");
    let msg = client.request_password_reset("alice@example.com".into()).await.unwrap();
    assert_eq!(msg.message, "sent");
    let msg = client.reset_password("token".into(), "newpass".into()).await.unwrap();
    assert_eq!(msg.message, "ok");
}

#[tokio::test]
async fn api_client_attendance_and_requests_endpoints_succeed() {
    let server = MockServer::start_async().await;

    server.mock(|when, then| {
        when.method(POST).path("/api/attendance/clock-in");
        then.status(200).json_body(attendance_json("att-1"));
    });
    server.mock(|when, then| {
        when.method(POST).path("/api/attendance/clock-out");
        then.status(200).json_body(attendance_json("att-1"));
    });
    server.mock(|when, then| {
        when.method(POST).path("/api/attendance/break-start");
        then.status(200).json_body(break_record_json("br-1"));
    });
    server.mock(|when, then| {
        when.method(POST).path("/api/attendance/break-end");
        then.status(200).json_body(break_record_json("br-1"));
    });
    server.mock(|when, then| {
        when.method(GET).path("/api/attendance/me");
        then.status(200).json_body(json!([attendance_json("att-1")]));
    });
    server.mock(|when, then| {
        when.method(GET).path("/api/attendance/status");
        then.status(200).json_body(attendance_status_json("clocked_in"));
    });
    server.mock(|when, then| {
        when.method(GET).path("/api/attendance/att-1/breaks");
        then.status(200).json_body(json!([break_record_json("br-1")]));
    });
    server.mock(|when, then| {
        when.method(PUT).path("/api/admin/attendance");
        then.status(200).json_body(attendance_json("att-1"));
    });
    server.mock(|when, then| {
        when.method(PUT).path("/api/admin/breaks/br-1/force-end");
        then.status(200).json_body(break_record_json("br-1"));
    });
    server.mock(|when, then| {
        when.method(GET).path("/api/attendance/me/summary");
        then.status(200).json_body(attendance_summary_json());
    });
    server.mock(|when, then| {
        when.method(GET).path("/api/attendance/export");
        then.status(200).json_body(json!({ "filename": "me.csv", "csv_data": "a,b\\n1,2" }));
    });

    server.mock(|when, then| {
        when.method(GET).path("/api/admin/requests");
        then.status(200).json_body(json!({ "items": [] }));
    });
    server.mock(|when, then| {
        when.method(PUT).path("/api/admin/requests/req-1/approve");
        then.status(200).json_body(json!({ "status": "approved" }));
    });
    server.mock(|when, then| {
        when.method(PUT).path("/api/admin/requests/req-1/reject");
        then.status(200).json_body(json!({ "status": "rejected" }));
    });
    server.mock(|when, then| {
        when.method(PUT).path("/api/requests/req-1");
        then.status(200).json_body(json!({ "status": "updated" }));
    });
    server.mock(|when, then| {
        when.method(DELETE).path("/api/requests/req-1");
        then.status(200).json_body(json!({ "status": "cancelled" }));
    });
    server.mock(|when, then| {
        when.method(POST).path("/api/requests/leave");
        then.status(200).json_body(leave_request_json("leave-1"));
    });
    server.mock(|when, then| {
        when.method(POST).path("/api/requests/overtime");
        then.status(200).json_body(overtime_request_json("ot-1"));
    });
    server.mock(|when, then| {
        when.method(GET).path("/api/requests/me");
        then.status(200).json_body(json!({ "items": [] }));
    });

    let client = api_client(&server);
    client.clock_in().await.unwrap();
    client.clock_out().await.unwrap();
    client.break_start("att-1").await.unwrap();
    client.break_end("br-1").await.unwrap();
    assert_eq!(client.get_my_attendance_range(None, None).await.unwrap().len(), 1);
    assert_eq!(client.get_attendance_status(None).await.unwrap().status, "clocked_in");
    assert_eq!(client.get_breaks_by_attendance("att-1").await.unwrap().len(), 1);
    client
        .admin_upsert_attendance(AdminAttendanceUpsert {
            user_id: "u1".into(),
            date: chrono::NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
            clock_in_time: chrono::NaiveDate::from_ymd_opt(2025, 1, 2)
                .unwrap()
                .and_hms_opt(9, 0, 0)
                .unwrap(),
            clock_out_time: None,
            breaks: None,
        })
        .await
        .unwrap();
    client.admin_force_end_break("br-1").await.unwrap();
    let summary = client.get_my_summary(Some(2025), Some(1)).await.unwrap();
    assert_eq!(summary.total_work_days, 20);
    let export = client
        .export_my_attendance_filtered(Some("2025-01-01"), Some("2025-01-31"))
        .await
        .unwrap();
    assert_eq!(export["filename"], "me.csv");

    client.admin_list_requests(None, None, None, None).await.unwrap();
    client.admin_approve_request("req-1", "ok").await.unwrap();
    client.admin_reject_request("req-1", "no").await.unwrap();
    client.update_request("req-1", json!({ "status": "updated" })).await.unwrap();
    client.cancel_request("req-1").await.unwrap();
    client.create_leave_request(CreateLeaveRequest {
        leave_type: "annual".into(),
        start_date: chrono::NaiveDate::from_ymd_opt(2025, 1, 10).unwrap(),
        end_date: chrono::NaiveDate::from_ymd_opt(2025, 1, 12).unwrap(),
        reason: None,
    }).await.unwrap();
    client.create_overtime_request(CreateOvertimeRequest {
        date: chrono::NaiveDate::from_ymd_opt(2025, 1, 11).unwrap(),
        planned_hours: 2.5,
        reason: None,
    }).await.unwrap();
    client.get_my_requests().await.unwrap();
}

#[tokio::test]
async fn api_client_subject_request_and_audit_log_endpoints_succeed() {
    let server = MockServer::start_async().await;

    server.mock(|when, then| {
        when.method(POST).path("/api/subject-requests");
        then.status(200).json_body(subject_request_json("sr-1"));
    });
    server.mock(|when, then| {
        when.method(GET).path("/api/subject-requests/me");
        then.status(200).json_body(json!([subject_request_json("sr-1")]));
    });
    server.mock(|when, then| {
        when.method(DELETE).path("/api/subject-requests/sr-1");
        then.status(200).json_body(json!({}));
    });
    server.mock(|when, then| {
        when.method(GET).path("/api/admin/subject-requests");
        then.status(200).json_body(json!({ "page": 1, "per_page": 20, "total": 1, "items": [subject_request_json("sr-1")] }));
    });
    server.mock(|when, then| {
        when.method(PUT).path("/api/admin/subject-requests/sr-1/approve");
        then.status(200).json_body(json!({}));
    });
    server.mock(|when, then| {
        when.method(PUT).path("/api/admin/subject-requests/sr-1/reject");
        then.status(200).json_body(json!({}));
    });

    server.mock(|when, then| {
        when.method(GET).path("/api/admin/audit-logs");
        then.status(200).json_body(json!({
            "page": 1,
            "per_page": 50,
            "total": 0,
            "items": []
        }));
    });
    server.mock(|when, then| {
        when.method(GET).path("/api/admin/audit-logs/export");
        then.status(200).json_body(json!([{
            "id": "log-1",
            "occurred_at": "2025-01-01T00:00:00Z",
            "actor_id": null,
            "actor_type": "system",
            "event_type": "export",
            "target_type": null,
            "target_id": null,
            "result": "success",
            "error_code": null,
            "metadata": null,
            "ip": null,
            "user_agent": null,
            "request_id": null
        }]));
    });

    let client = api_client(&server);
    client
        .create_subject_request(CreateDataSubjectRequest {
            request_type: DataSubjectRequestType::Access,
            details: None,
        })
        .await
        .unwrap();
    client.list_my_subject_requests().await.unwrap();
    client.cancel_subject_request("sr-1").await.unwrap();
    client
        .admin_list_subject_requests(None, None, None, None, None, 1, 50)
        .await
        .unwrap();
    client.admin_approve_subject_request("sr-1", "ok").await.unwrap();
    client.admin_reject_subject_request("sr-1", "no").await.unwrap();

    client
        .list_audit_logs(1, 50, None, None, None, None, None)
        .await
        .unwrap();
    client
        .export_audit_logs(None, None, None, None, None)
        .await
        .unwrap();
}

#[tokio::test]
async fn api_client_auth_login_and_refresh_use_test_overrides() {
    let server = MockServer::start_async().await;

    server.mock(|when, then| {
        when.method(POST).path("/api/auth/login");
        then.status(200).json_body(json!({ "user": user_json("u1") }));
    });
    server.mock(|when, then| {
        when.method(POST).path("/api/auth/logout");
        then.status(200).json_body(json!({}));
    });
    server.mock(|when, then| {
        when.method(GET).path("/api/auth/mfa");
        then.status(200).json_body(json!({ "enabled": false, "pending": false }));
    });
    server.mock(|when, then| {
        when.method(POST).path("/api/auth/mfa/register");
        then.status(200).json_body(json!({ "secret": "secret", "otpauth_url": "otpauth://test" }));
    });
    server.mock(|when, then| {
        when.method(POST).path("/api/auth/mfa/activate");
        then.status(200).json_body(json!({}));
    });
    server.mock(|when, then| {
        when.method(PUT).path("/api/auth/change-password");
        then.status(200).json_body(json!({}));
    });

    let client = api_client(&server);
    let login = client
        .login(LoginRequest {
            username: "alice".into(),
            password: "pass".into(),
            totp_code: None,
            device_label: Some("test-device".into()),
        })
        .await
        .unwrap();
    assert_eq!(login.user.id, "u1");

    super::auth::queue_refresh_override(Ok(LoginResponse { user: UserResponse {
        id: "u1".into(),
        username: "alice".into(),
        full_name: "Alice Example".into(),
        role: "admin".into(),
        is_system_admin: true,
        mfa_enabled: false,
    }}));
    let _ = client.refresh_token().await.unwrap();

    client.logout(false).await.unwrap();
    client.get_mfa_status().await.unwrap();
    client.register_mfa().await.unwrap();
    client.activate_mfa("123456").await.unwrap();
    client.change_password("old".into(), "newpass".into()).await.unwrap();
}

#[tokio::test]
async fn api_client_handles_unauthorized_with_refresh_override() {
    let server = MockServer::start_async().await;

    server.mock(|when, then| {
        when.method(GET).path("/api/auth/me");
        then.status(401).json_body(json!({ "error": "unauthorized", "code": "UNAUTHORIZED" }));
    });

    let client = api_client(&server);
    super::auth::queue_refresh_override(Ok(LoginResponse { user: UserResponse {
        id: "u1".into(),
        username: "alice".into(),
        full_name: "Alice Example".into(),
        role: "admin".into(),
        is_system_admin: true,
        mfa_enabled: false,
    }}));
    let err = client.get_me().await.unwrap_err();
    assert_eq!(err.code, "UNAUTHORIZED");
}
