use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::NaiveDate;
use serde_json::Value;
use timekeeper_backend::{
    handlers::{
        admin::{
            approve_request, list_requests, reject_request, ApprovePayload, RejectPayload,
            RequestListQuery,
        },
        requests::get_my_requests,
    },
    models::{leave_request::LeaveType, user::UserRole},
};

mod support;
use support::{seed_leave_request, seed_overtime_request, seed_user, setup_test_pool, test_config};

fn init_tracing() {
    let _ = tracing_subscriber::fmt::try_init();
}

#[tokio::test]
async fn get_my_requests_returns_leave_and_overtime() {
    let Some(pool) = setup_test_pool().await else {
        eprintln!("Skipping get_my_requests_returns_leave_and_overtime: database unavailable");
        return;
    };
    init_tracing();
    let config = test_config();
    let user = seed_user(&pool, UserRole::Employee, false).await;
    let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    seed_leave_request(&pool, &user.id, LeaveType::Annual, date, date).await;
    seed_overtime_request(&pool, &user.id, date, 1.5).await;

    let response =
        get_my_requests(State((pool.clone_pool(), config)), Extension(user.clone())).await;

    let payload: Value = response.expect("get_my_requests ok").0;
    let leave = payload
        .get("leave_requests")
        .and_then(|v| v.as_array())
        .expect("leave list");
    let overtime = payload
        .get("overtime_requests")
        .and_then(|v| v.as_array())
        .expect("overtime list");
    assert_eq!(leave.len(), 1);
    assert_eq!(overtime.len(), 1);
}

#[tokio::test]
async fn admin_list_requests_includes_seeded_records() {
    let Some(pool) = setup_test_pool().await else {
        eprintln!("Skipping admin_list_requests_includes_seeded_records: database unavailable");
        return;
    };
    init_tracing();
    let config = test_config();
    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let date = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();
    seed_leave_request(&pool, &employee.id, LeaveType::Sick, date, date).await;
    seed_overtime_request(&pool, &employee.id, date, 2.0).await;

    let query = RequestListQuery {
        status: None,
        r#type: None,
        user_id: None,
        from: None,
        to: None,
        page: Some(1),
        per_page: Some(20),
    };

    let response = list_requests(
        State((pool.clone_pool(), config)),
        Extension(admin.clone()),
        Query(query),
    )
    .await;

    let payload: Value = response.expect("list_requests ok").0;
    let leave = payload
        .get("leave_requests")
        .and_then(|v| v.as_array())
        .expect("leave array");
    let overtime = payload
        .get("overtime_requests")
        .and_then(|v| v.as_array())
        .expect("overtime array");
    assert!(!leave.is_empty());
    assert!(!overtime.is_empty());
}

#[sqlx::test(migrations = "./migrations")]
async fn approve_request_rejects_comment_longer_than_500(pool: PgPool) {
    init_tracing();
    let config = test_config();
    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let payload = ApprovePayload {
        comment: "a".repeat(501),
    };

    let error = approve_request(
        State((pool.clone(), config)),
        Extension(admin),
        Path("request-id".to_string()),
        Json(payload),
    )
    .await
    .expect_err("comment too long should error");

    let (status, Json(body)) = error;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body.get("error").and_then(|v| v.as_str()),
        Some("comment must be between 1 and 500 characters"),
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn reject_request_rejects_comment_longer_than_500(pool: PgPool) {
    init_tracing();
    let config = test_config();
    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let payload = RejectPayload {
        comment: "b".repeat(600),
    };

    let error = reject_request(
        State((pool.clone(), config)),
        Extension(admin),
        Path("request-id".to_string()),
        Json(payload),
    )
    .await
    .expect_err("comment too long should error");

    let (status, Json(body)) = error;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body.get("error").and_then(|v| v.as_str()),
        Some("comment must be between 1 and 500 characters"),
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn admin_list_requests_rejects_page_overflow(pool: PgPool) {
    init_tracing();
    let config = test_config();
    let admin = seed_user(&pool, UserRole::Admin, false).await;

    let query = RequestListQuery {
        status: None,
        r#type: None,
        user_id: None,
        from: None,
        to: None,
        page: Some(i64::MAX),
        per_page: Some(100),
    };

    let error = list_requests(
        State((pool.clone(), config)),
        Extension(admin),
        Query(query),
    )
    .await
    .expect_err("page overflow should error");

    let (status, Json(body)) = error;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body.get("error").and_then(|v| v.as_str()),
        Some("page is too large")
    );
}
