use axum::{
    body::Body,
    http::{Request, StatusCode},
    Extension, Router,
};
use chrono::{Duration, NaiveDate, NaiveDateTime};
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use timekeeper_backend::{
    handlers::{
        admin::attendance_correction_requests as admin_corrections, attendance,
        attendance_correction_requests as user_corrections, requests as user_requests,
    },
    models::user::{User, UserRole},
    state::AppState,
};
use tower::ServiceExt;
use uuid::Uuid;

mod support;

use support::{
    create_test_token, seed_attendance, seed_break_record, seed_user, test_config, test_pool,
};

async fn integration_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static GUARD: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();
    GUARD
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

fn user_router(pool: PgPool, user: User) -> Router {
    let state = AppState::new(pool, None, None, None, test_config());
    Router::new()
        .route(
            "/api/attendance-corrections",
            axum::routing::post(user_corrections::create_attendance_correction_request),
        )
        .route(
            "/api/attendance-corrections/me",
            axum::routing::get(user_corrections::list_my_attendance_correction_requests),
        )
        .route(
            "/api/attendance-corrections/{id}",
            axum::routing::put(user_corrections::update_my_attendance_correction_request)
                .delete(user_corrections::cancel_my_attendance_correction_request),
        )
        .route(
            "/api/attendance/me",
            axum::routing::get(attendance::get_my_attendance),
        )
        .route(
            "/api/requests/{id}",
            axum::routing::put(user_requests::update_request).delete(user_requests::cancel_request),
        )
        .layer(Extension(user))
        .with_state(state)
}

fn admin_router(pool: PgPool, user: User) -> Router {
    let state = AppState::new(pool, None, None, None, test_config());
    Router::new()
        .route(
            "/api/admin/attendance-corrections",
            axum::routing::get(admin_corrections::list_attendance_correction_requests),
        )
        .route(
            "/api/admin/attendance-corrections/{id}",
            axum::routing::get(admin_corrections::get_attendance_correction_request_detail),
        )
        .route(
            "/api/admin/attendance-corrections/{id}/approve",
            axum::routing::put(admin_corrections::approve_attendance_correction_request),
        )
        .route(
            "/api/admin/attendance-corrections/{id}/reject",
            axum::routing::put(admin_corrections::reject_attendance_correction_request),
        )
        .layer(Extension(user))
        .with_state(state)
}

async fn response_json(response: axum::response::Response) -> serde_json::Value {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read response body");
    serde_json::from_slice(&body).expect("parse json body")
}

#[tokio::test]
async fn employee_can_create_update_and_cancel_attendance_correction() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let token = create_test_token(employee.id, employee.role.clone());
    let app = user_router(pool.clone(), employee.clone());

    let date = NaiveDate::from_ymd_opt(2026, 2, 10).expect("valid date");
    let clock_in = NaiveDateTime::new(date, chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap());
    let clock_out = NaiveDateTime::new(date, chrono::NaiveTime::from_hms_opt(18, 0, 0).unwrap());
    let attendance =
        seed_attendance(&pool, employee.id, date, Some(clock_in), Some(clock_out)).await;
    let break_start = clock_in + Duration::hours(3);
    let break_end = break_start + Duration::minutes(45);
    seed_break_record(&pool, attendance.id, break_start, Some(break_end)).await;

    let create_payload = json!({
        "date": date.to_string(),
        "clock_out_time": (clock_out + Duration::minutes(30)).format("%Y-%m-%dT%H:%M:%S").to_string(),
        "breaks": [
            {
                "break_start_time": break_start.format("%Y-%m-%dT%H:%M:%S").to_string(),
                "break_end_time": break_end.format("%Y-%m-%dT%H:%M:%S").to_string()
            }
        ],
        "reason": "退勤打刻漏れのため修正"
    });
    let create_req = Request::builder()
        .method("POST")
        .uri("/api/attendance-corrections")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .expect("build create request");

    let create_res = app.clone().oneshot(create_req).await.expect("create call");
    assert_eq!(create_res.status(), StatusCode::OK);
    let create_json = response_json(create_res).await;
    let request_id = create_json["id"].as_str().expect("request id").to_string();
    assert_eq!(create_json["status"], "pending");

    let update_payload = json!({
        "clock_out_time": (clock_out + Duration::minutes(60)).format("%Y-%m-%dT%H:%M:%S").to_string(),
        "reason": "退勤時刻を再修正"
    });
    let update_req = Request::builder()
        .method("PUT")
        .uri(format!("/api/attendance-corrections/{request_id}"))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(update_payload.to_string()))
        .expect("build update request");
    let update_res = app.clone().oneshot(update_req).await.expect("update call");
    assert_eq!(update_res.status(), StatusCode::OK);

    let cancel_req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/attendance-corrections/{request_id}"))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("build cancel request");
    let cancel_res = app.oneshot(cancel_req).await.expect("cancel call");
    assert_eq!(cancel_res.status(), StatusCode::OK);
}

#[tokio::test]
async fn admin_approval_sets_request_status_and_effective_values() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let user_app = user_router(pool.clone(), employee.clone());
    let admin_app = admin_router(pool.clone(), admin.clone());
    let user_token = create_test_token(employee.id, employee.role.clone());
    let admin_token = create_test_token(admin.id, admin.role.clone());

    let date = NaiveDate::from_ymd_opt(2026, 2, 11).expect("valid date");
    let clock_in = NaiveDateTime::new(date, chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap());
    let clock_out = NaiveDateTime::new(date, chrono::NaiveTime::from_hms_opt(18, 0, 0).unwrap());
    let attendance =
        seed_attendance(&pool, employee.id, date, Some(clock_in), Some(clock_out)).await;

    let create_payload = json!({
        "date": date.to_string(),
        "clock_out_time": (clock_out + Duration::minutes(20)).format("%Y-%m-%dT%H:%M:%S").to_string(),
        "reason": "退勤修正"
    });
    let create_req = Request::builder()
        .method("POST")
        .uri("/api/attendance-corrections")
        .header("Authorization", format!("Bearer {}", user_token))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .expect("build create request");
    let create_res = user_app
        .clone()
        .oneshot(create_req)
        .await
        .expect("create call");
    assert_eq!(create_res.status(), StatusCode::OK);
    let create_json = response_json(create_res).await;
    let request_id = create_json["id"].as_str().expect("request id");

    let approve_payload = json!({ "comment": "承認" });
    let approve_req = Request::builder()
        .method("PUT")
        .uri(format!(
            "/api/admin/attendance-corrections/{request_id}/approve"
        ))
        .header("Authorization", format!("Bearer {}", admin_token))
        .header("Content-Type", "application/json")
        .body(Body::from(approve_payload.to_string()))
        .expect("build approve request");
    let approve_res = admin_app.oneshot(approve_req).await.expect("approve call");
    assert_eq!(approve_res.status(), StatusCode::OK);

    let status_row: (String,) =
        sqlx::query_as("SELECT status FROM attendance_correction_requests WHERE id = $1")
            .bind(request_id)
            .fetch_one(&pool)
            .await
            .expect("load request status");
    assert_eq!(status_row.0, "approved");

    let effective_row: (String,) = sqlx::query_as(
        "SELECT source_request_id FROM attendance_correction_effective_values WHERE attendance_id = $1",
    )
    .bind(attendance.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("load effective values");
    assert_eq!(effective_row.0, request_id);
}

#[tokio::test]
async fn admin_cannot_approve_own_attendance_correction_request() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let user_app = user_router(pool.clone(), admin.clone());
    let admin_app = admin_router(pool.clone(), admin.clone());
    let admin_token = create_test_token(admin.id, admin.role.clone());

    let date = NaiveDate::from_ymd_opt(2026, 2, 11).expect("valid date");
    let clock_in = NaiveDateTime::new(date, chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap());
    let clock_out = NaiveDateTime::new(date, chrono::NaiveTime::from_hms_opt(18, 0, 0).unwrap());
    seed_attendance(&pool, admin.id, date, Some(clock_in), Some(clock_out)).await;

    let create_payload = json!({
        "date": date.to_string(),
        "clock_out_time": (clock_out + Duration::minutes(20)).format("%Y-%m-%dT%H:%M:%S").to_string(),
        "reason": "自分の申請"
    });
    let create_req = Request::builder()
        .method("POST")
        .uri("/api/attendance-corrections")
        .header("Authorization", format!("Bearer {}", admin_token))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .expect("build create request");
    let create_res = user_app.oneshot(create_req).await.expect("create call");
    assert_eq!(create_res.status(), StatusCode::OK);
    let create_json = response_json(create_res).await;
    let request_id = create_json["id"].as_str().expect("request id");

    let approve_payload = json!({ "comment": "自分で承認" });
    let approve_req = Request::builder()
        .method("PUT")
        .uri(format!(
            "/api/admin/attendance-corrections/{request_id}/approve"
        ))
        .header("Authorization", format!("Bearer {}", admin_token))
        .header("Content-Type", "application/json")
        .body(Body::from(approve_payload.to_string()))
        .expect("build approve request");
    let approve_res = admin_app.oneshot(approve_req).await.expect("approve call");
    assert_eq!(approve_res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn admin_cannot_reject_own_attendance_correction_request() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let user_app = user_router(pool.clone(), admin.clone());
    let admin_app = admin_router(pool.clone(), admin.clone());
    let admin_token = create_test_token(admin.id, admin.role.clone());

    let date = NaiveDate::from_ymd_opt(2026, 2, 11).expect("valid date");
    let clock_in = NaiveDateTime::new(date, chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap());
    let clock_out = NaiveDateTime::new(date, chrono::NaiveTime::from_hms_opt(18, 0, 0).unwrap());
    seed_attendance(&pool, admin.id, date, Some(clock_in), Some(clock_out)).await;

    let create_payload = json!({
        "date": date.to_string(),
        "clock_out_time": (clock_out + Duration::minutes(20)).format("%Y-%m-%dT%H:%M:%S").to_string(),
        "reason": "自分の申請"
    });
    let create_req = Request::builder()
        .method("POST")
        .uri("/api/attendance-corrections")
        .header("Authorization", format!("Bearer {}", admin_token))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .expect("build create request");
    let create_res = user_app.oneshot(create_req).await.expect("create call");
    assert_eq!(create_res.status(), StatusCode::OK);
    let create_json = response_json(create_res).await;
    let request_id = create_json["id"].as_str().expect("request id");

    let reject_payload = json!({ "comment": "自分で却下" });
    let reject_req = Request::builder()
        .method("PUT")
        .uri(format!(
            "/api/admin/attendance-corrections/{request_id}/reject"
        ))
        .header("Authorization", format!("Bearer {}", admin_token))
        .header("Content-Type", "application/json")
        .body(Body::from(reject_payload.to_string()))
        .expect("build reject request");
    let reject_res = admin_app.oneshot(reject_req).await.expect("reject call");
    assert_eq!(reject_res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn admin_approval_fails_with_conflict_when_attendance_changed() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let user_app = user_router(pool.clone(), employee.clone());
    let admin_app = admin_router(pool.clone(), admin.clone());
    let user_token = create_test_token(employee.id, employee.role.clone());
    let admin_token = create_test_token(admin.id, admin.role.clone());

    let date = NaiveDate::from_ymd_opt(2026, 2, 12).expect("valid date");
    let clock_in = NaiveDateTime::new(date, chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap());
    let clock_out = NaiveDateTime::new(date, chrono::NaiveTime::from_hms_opt(18, 0, 0).unwrap());
    let attendance =
        seed_attendance(&pool, employee.id, date, Some(clock_in), Some(clock_out)).await;

    let create_payload = json!({
        "date": date.to_string(),
        "clock_out_time": (clock_out + Duration::minutes(15)).format("%Y-%m-%dT%H:%M:%S").to_string(),
        "reason": "退勤修正"
    });
    let create_req = Request::builder()
        .method("POST")
        .uri("/api/attendance-corrections")
        .header("Authorization", format!("Bearer {}", user_token))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .expect("build create request");
    let create_res = user_app
        .clone()
        .oneshot(create_req)
        .await
        .expect("create call");
    assert_eq!(create_res.status(), StatusCode::OK);
    let create_json = response_json(create_res).await;
    let request_id = create_json["id"].as_str().expect("request id");

    sqlx::query("UPDATE attendance SET clock_out_time = $1, updated_at = NOW() WHERE id = $2")
        .bind(clock_out + Duration::minutes(5))
        .bind(attendance.id.to_string())
        .execute(&pool)
        .await
        .expect("mutate attendance");

    let approve_payload = json!({ "comment": "承認" });
    let approve_req = Request::builder()
        .method("PUT")
        .uri(format!(
            "/api/admin/attendance-corrections/{request_id}/approve"
        ))
        .header("Authorization", format!("Bearer {}", admin_token))
        .header("Content-Type", "application/json")
        .body(Body::from(approve_payload.to_string()))
        .expect("build approve request");
    let approve_res = admin_app.oneshot(approve_req).await.expect("approve call");
    assert_eq!(approve_res.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn update_request_returns_internal_error_when_correction_table_is_missing() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let app = user_router(pool.clone(), employee.clone());
    let token = create_test_token(employee.id, employee.role.clone());

    sqlx::query(
        "ALTER TABLE attendance_correction_requests RENAME TO attendance_correction_requests_tmp",
    )
    .execute(&pool)
    .await
    .expect("rename correction table");

    let request_id = Uuid::new_v4().to_string();
    let payload = json!({
        "reason": "テーブル欠落時の確認",
        "proposed_values": {
            "clock_out_time": "2026-02-12T18:30:00",
            "breaks": []
        }
    });
    let request = Request::builder()
        .method("PUT")
        .uri(format!("/api/requests/{request_id}"))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .expect("build update request");

    let response = app.oneshot(request).await.expect("send request");

    sqlx::query(
        "ALTER TABLE attendance_correction_requests_tmp RENAME TO attendance_correction_requests",
    )
    .execute(&pool)
    .await
    .expect("restore correction table");

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn concurrent_approval_stress_allows_only_one_success() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let user_app = user_router(pool.clone(), employee.clone());
    let admin_app = admin_router(pool.clone(), admin.clone());
    let user_token = create_test_token(employee.id, employee.role.clone());
    let admin_token = create_test_token(admin.id, admin.role.clone());

    let date = NaiveDate::from_ymd_opt(2026, 2, 13).expect("valid date");
    let clock_in = NaiveDateTime::new(date, chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap());
    let clock_out = NaiveDateTime::new(date, chrono::NaiveTime::from_hms_opt(18, 0, 0).unwrap());
    seed_attendance(&pool, employee.id, date, Some(clock_in), Some(clock_out)).await;

    let create_payload = json!({
        "date": date.to_string(),
        "clock_out_time": (clock_out + Duration::minutes(25)).format("%Y-%m-%dT%H:%M:%S").to_string(),
        "reason": "同時承認テスト"
    });
    let create_req = Request::builder()
        .method("POST")
        .uri("/api/attendance-corrections")
        .header("Authorization", format!("Bearer {}", user_token))
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .expect("build create request");
    let create_res = user_app.oneshot(create_req).await.expect("create call");
    assert_eq!(create_res.status(), StatusCode::OK);
    let create_json = response_json(create_res).await;
    let request_id = create_json["id"].as_str().expect("request id").to_string();

    let workers = 24usize;
    let barrier = Arc::new(tokio::sync::Barrier::new(workers));
    let mut handles = Vec::with_capacity(workers);
    for _ in 0..workers {
        let app = admin_app.clone();
        let token = admin_token.clone();
        let rid = request_id.clone();
        let barrier = barrier.clone();
        handles.push(tokio::spawn(async move {
            barrier.wait().await;
            let approve_payload = json!({ "comment": "stress approve" });
            let approve_req = Request::builder()
                .method("PUT")
                .uri(format!("/api/admin/attendance-corrections/{rid}/approve"))
                .header("Authorization", format!("Bearer {}", token))
                .header("Content-Type", "application/json")
                .body(Body::from(approve_payload.to_string()))
                .expect("build approve request");
            app.oneshot(approve_req)
                .await
                .expect("approve call")
                .status()
        }));
    }

    let mut ok_count = 0usize;
    let mut conflict_count = 0usize;
    for handle in handles {
        let status = handle.await.expect("join approval task");
        if status == StatusCode::OK {
            ok_count += 1;
        } else if status == StatusCode::CONFLICT {
            conflict_count += 1;
        } else {
            panic!("unexpected status from concurrent approval: {status}");
        }
    }
    assert_eq!(ok_count, 1, "only one approval must succeed");
    assert_eq!(
        conflict_count,
        workers - 1,
        "remaining approvals must conflict"
    );

    let row: (String, String, String) = sqlx::query_as(
        "SELECT status, approved_by, decision_comment
         FROM attendance_correction_requests
         WHERE id = $1",
    )
    .bind(&request_id)
    .fetch_one(&pool)
    .await
    .expect("fetch correction request");
    assert_eq!(row.0, "approved");
    assert_eq!(row.1, admin.id.to_string());
    assert_eq!(row.2, "stress approve");

    let effective: (String, String) = sqlx::query_as(
        "SELECT source_request_id, applied_by
         FROM attendance_correction_effective_values
         WHERE source_request_id = $1",
    )
    .bind(&request_id)
    .fetch_one(&pool)
    .await
    .expect("fetch effective values");
    assert_eq!(effective.0, request_id);
    assert_eq!(effective.1, admin.id.to_string());
}
