use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::{delete, get, post, put},
    Extension, Router,
};
use chrono::NaiveDate;
use serde_json::{json, Value};
use sqlx::PgPool;
use timekeeper_backend::{
    handlers::{admin, leave_ledger, requests},
    models::user::{User, UserRole},
    state::AppState,
};
use tower::ServiceExt;
use uuid::Uuid;

mod support;

use support::{integration_guard, seed_user, test_config, test_pool};

fn router(pool: PgPool, user: User) -> Router {
    let state = AppState::new(pool, None, None, None, test_config());
    Router::new()
        .route("/api/requests/leave", post(requests::create_leave_request))
        .route("/api/requests/{id}", delete(requests::cancel_request))
        .route(
            "/api/admin/requests/{id}/approve",
            put(admin::approve_request),
        )
        .route(
            "/api/leave-balances/me",
            get(leave_ledger::get_my_leave_balance),
        )
        .layer(Extension(state.clone()))
        .layer(Extension(user))
        .with_state(state)
}

async fn request_json(
    app: Router,
    method: &str,
    uri: &str,
    payload: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    let body = if let Some(value) = payload {
        builder = builder.header("Content-Type", "application/json");
        Body::from(value.to_string())
    } else {
        Body::empty()
    };
    let response = app
        .oneshot(builder.body(body).expect("request"))
        .await
        .expect("response");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes)
            .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()))
    };
    (status, body)
}

async fn seed_annual_balance(pool: &PgPool, user: &User, days: i32) {
    let lot_id = Uuid::new_v4();
    let granted_at = NaiveDate::from_ymd_opt(2026, 1, 1).expect("date");
    let expires_at = NaiveDate::from_ymd_opt(2028, 1, 1).expect("date");
    sqlx::query(
        "INSERT INTO leave_ledger_entries (
            id, user_id, leave_type, kind, lot_id, amount_minutes,
            day_equivalent_minutes, granted_at, expires_at, grant_base_date,
            reason, effective_at
         ) VALUES ($1,$2,'annual','adjust',$3,$4,480,$5,$6,$5,'test balance seed',$5)",
    )
    .bind(Uuid::new_v4())
    .bind(user.id.to_string())
    .bind(lot_id)
    .bind(days * 480)
    .bind(granted_at)
    .bind(expires_at)
    .execute(pool)
    .await
    .expect("seed annual balance");
}

async fn balance_minutes(pool: PgPool, user: User, as_of: &str) -> i64 {
    let uri = format!("/api/leave-balances/me?as_of={as_of}");
    let (status, body) = request_json(router(pool, user), "GET", &uri, None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["available_minutes"].as_i64().expect("minutes")
}

#[tokio::test]
async fn annual_leave_request_rejects_insufficient_balance_at_submission() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");
    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let (status, body) = request_json(
        router(pool, employee),
        "POST",
        "/api/requests/leave",
        Some(json!({
            "leave_type": "annual",
            "start_date": "2026-07-10",
            "end_date": "2026-07-11",
            "reason": "vacation"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["code"], "LEAVE_BALANCE_INSUFFICIENT");
}

#[tokio::test]
async fn annual_leave_approval_consumes_and_approved_cancel_releases_balance() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");
    let manager = seed_user(&pool, UserRole::Manager, true).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    seed_annual_balance(&pool, &employee, 5).await;

    let (status, body) = request_json(
        router(pool.clone(), employee.clone()),
        "POST",
        "/api/requests/leave",
        Some(json!({
            "leave_type": "annual",
            "start_date": "2026-07-10",
            "end_date": "2026-07-12",
            "reason": "vacation"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let request_id = body["id"].as_str().expect("request id").to_string();

    assert_eq!(
        balance_minutes(pool.clone(), employee.clone(), "2026-07-12").await,
        2400,
        "pending annual leave must not reserve balance"
    );

    let approve_path = format!("/api/admin/requests/{request_id}/approve");
    let (status, body) = request_json(
        router(pool.clone(), manager),
        "PUT",
        &approve_path,
        Some(json!({ "comment": "approved" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        balance_minutes(pool.clone(), employee.clone(), "2026-07-12").await,
        960
    );

    let consume_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM leave_ledger_entries
         WHERE leave_request_id = $1 AND kind = 'consume'",
    )
    .bind(&request_id)
    .fetch_one(&pool)
    .await
    .expect("consume count");
    assert_eq!(consume_count, 1);

    let cancel_path = format!("/api/requests/{request_id}");
    let (status, body) = request_json(
        router(pool.clone(), employee.clone()),
        "DELETE",
        &cancel_path,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        balance_minutes(pool.clone(), employee, "2026-07-12").await,
        2400
    );

    let release_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM leave_ledger_entries
         WHERE leave_request_id = $1 AND kind = 'release'",
    )
    .bind(&request_id)
    .fetch_one(&pool)
    .await
    .expect("release count");
    assert_eq!(release_count, 1);
}

#[tokio::test]
async fn non_annual_leave_stays_balance_independent() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");
    let manager = seed_user(&pool, UserRole::Manager, true).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let (status, body) = request_json(
        router(pool.clone(), employee),
        "POST",
        "/api/requests/leave",
        Some(json!({
            "leave_type": "sick",
            "start_date": "2026-07-10",
            "end_date": "2026-07-12"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let request_id = body["id"].as_str().expect("request id").to_string();

    let approve_path = format!("/api/admin/requests/{request_id}/approve");
    let (status, body) = request_json(
        router(pool, manager),
        "PUT",
        &approve_path,
        Some(json!({ "comment": "approved" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}
