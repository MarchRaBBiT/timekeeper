use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    routing::{delete, get, post, put},
    Extension, Router,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use std::sync::OnceLock;
use timekeeper_backend::{
    handlers::{admin, subject_requests},
    models::{
        request::RequestStatus,
        subject_request::{DataSubjectRequest, DataSubjectRequestResponse, DataSubjectRequestType},
        user::UserRole,
    },
    repositories::subject_request,
    state::AppState,
};
use tokio::sync::Mutex;
use tower::ServiceExt;

#[path = "support/mod.rs"]
mod support;

async fn integration_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD.get_or_init(|| Mutex::new(())).lock().await
}

async fn reset_subject_requests(pool: &PgPool) {
    sqlx::query("TRUNCATE subject_requests")
        .execute(pool)
        .await
        .expect("truncate subject_requests");
}

#[derive(Deserialize)]
struct AdminListResponse {
    total: i64,
    items: Vec<DataSubjectRequestResponse>,
}

#[tokio::test]
async fn create_and_list_subject_requests_for_user() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_subject_requests(&pool).await;

    let user = support::seed_user(&pool, UserRole::Employee, false).await;
    let state = AppState::new(pool.clone(), None, support::test_config());

    let app = Router::new()
        .route(
            "/api/subject-requests",
            post(subject_requests::create_subject_request),
        )
        .route(
            "/api/subject-requests/me",
            get(subject_requests::list_my_subject_requests),
        )
        .layer(Extension(user.clone()))
        .with_state(state);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/subject-requests")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "request_type": "access",
                        "details": "please export"
                    })
                    .to_string(),
                ))
                .expect("build request"),
        )
        .await
        .expect("call app");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 1024 * 64)
        .await
        .expect("read body");
    let created: DataSubjectRequestResponse =
        serde_json::from_slice(&body).expect("parse response");
    assert_eq!(created.user_id, user.id.to_string());
    assert!(matches!(
        created.request_type,
        DataSubjectRequestType::Access
    ));
    assert!(matches!(created.status, RequestStatus::Pending));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/subject-requests/me")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("call app");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 1024 * 64)
        .await
        .expect("read body");
    let list: Vec<DataSubjectRequestResponse> = serde_json::from_slice(&body).expect("parse list");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, created.id);
}

#[tokio::test]
async fn cancel_subject_request_marks_status() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_subject_requests(&pool).await;

    let user = support::seed_user(&pool, UserRole::Employee, false).await;
    let state = AppState::new(pool.clone(), None, support::test_config());
    let now = Utc::now();

    let request = DataSubjectRequest::new(
        user.id.to_string(),
        DataSubjectRequestType::Delete,
        Some("delete".to_string()),
        now,
    );
    subject_request::insert_subject_request(&pool, &request)
        .await
        .expect("insert request");

    let app = Router::new()
        .route(
            "/api/subject-requests/{id}",
            delete(subject_requests::cancel_subject_request),
        )
        .layer(Extension(user))
        .with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/subject-requests/{}", request.id))
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("call app");

    assert_eq!(response.status(), StatusCode::OK);

    let updated = subject_request::fetch_subject_request(&pool, &request.id)
        .await
        .expect("fetch request")
        .expect("request exists");
    assert!(matches!(updated.status, RequestStatus::Cancelled));
    assert!(updated.cancelled_at.is_some());
}

#[tokio::test]
async fn admin_can_list_and_decide_subject_requests() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_subject_requests(&pool).await;

    let user = support::seed_user(&pool, UserRole::Employee, false).await;
    let admin_user = support::seed_user(&pool, UserRole::Admin, false).await;
    let state = AppState::new(pool.clone(), None, support::test_config());
    let now = Utc::now();

    let pending = DataSubjectRequest::new(
        user.id.to_string(),
        DataSubjectRequestType::Access,
        None,
        now,
    );
    subject_request::insert_subject_request(&pool, &pending)
        .await
        .expect("insert request");

    let admin_id = admin_user.id.to_string();

    let app = Router::new()
        .route(
            "/api/admin/subject-requests",
            get(admin::list_subject_requests),
        )
        .route(
            "/api/admin/subject-requests/{id}/approve",
            put(admin::approve_subject_request),
        )
        .route(
            "/api/admin/subject-requests/{id}/reject",
            put(admin::reject_subject_request),
        )
        .layer(Extension(admin_user.clone()))
        .with_state(state);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/admin/subject-requests?status=pending")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("call app");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 1024 * 64)
        .await
        .expect("read body");
    let list: AdminListResponse = serde_json::from_slice(&body).expect("parse list");
    assert_eq!(list.total, 1);
    assert_eq!(list.items.len(), 1);
    assert_eq!(list.items[0].id, pending.id);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/api/admin/subject-requests/{}/approve",
                    pending.id
                ))
                .header("content-type", "application/json")
                .body(Body::from(json!({"comment": "ok"}).to_string()))
                .expect("build request"),
        )
        .await
        .expect("call app");

    assert_eq!(response.status(), StatusCode::OK);

    let updated = subject_request::fetch_subject_request(&pool, &pending.id)
        .await
        .expect("fetch request")
        .expect("request exists");
    assert!(matches!(updated.status, RequestStatus::Approved));
    assert_eq!(updated.approved_by.as_deref(), Some(admin_id.as_str()));

    let rejected_request =
        DataSubjectRequest::new(user.id.to_string(), DataSubjectRequestType::Stop, None, now);
    subject_request::insert_subject_request(&pool, &rejected_request)
        .await
        .expect("insert request");

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/api/admin/subject-requests/{}/reject",
                    rejected_request.id
                ))
                .header("content-type", "application/json")
                .body(Body::from(json!({"comment": "no"}).to_string()))
                .expect("build request"),
        )
        .await
        .expect("call app");

    assert_eq!(response.status(), StatusCode::OK);

    let updated = subject_request::fetch_subject_request(&pool, &rejected_request.id)
        .await
        .expect("fetch request")
        .expect("request exists");
    assert!(matches!(updated.status, RequestStatus::Rejected));
    assert_eq!(updated.rejected_by.as_deref(), Some(admin_id.as_str()));
}
