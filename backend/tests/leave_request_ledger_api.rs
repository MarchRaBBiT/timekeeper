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

use support::{
    integration_guard, seed_user, seed_weekday_work_schedule_for_user, test_config, test_pool,
};

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
    seed_weekday_work_schedule_for_user(&pool, employee.id, "follow_weekly_pattern").await;
    // 1日分(480分)しか残高が無い状態で、稼働日2日分(金・月=960分)を申請する。
    seed_annual_balance(&pool, &employee, 1).await;

    // 2026-07-10 (Fri) - 2026-07-13 (Mon): 4 暦日だが稼働日は金・月の 2 日
    // (960分)。残高 480 分では不足として拒否される。
    let (status, body) = request_json(
        router(pool, employee),
        "POST",
        "/api/requests/leave",
        Some(json!({
            "leave_type": "annual",
            "start_date": "2026-07-10",
            "end_date": "2026-07-13",
            "reason": "vacation"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["code"], "LEAVE_BALANCE_INSUFFICIENT");
}

#[tokio::test]
async fn annual_leave_request_rejects_when_no_active_lot_exists() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    seed_weekday_work_schedule_for_user(&pool, employee.id, "follow_weekly_pattern").await;

    // 有給が一度も付与・移行されていないユーザー（アクティブなロットが無い）。
    // 旧実装は day_equivalent_minutes を 480 分に決め打ちフォールバックしていたが、
    // H-1 修正でロットが無い場合は明示的なエラーにする。
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
    assert_eq!(body["code"], "LEAVE_REQUEST_NO_ACTIVE_LOT");
}

#[tokio::test]
async fn annual_leave_request_rejects_when_no_working_days_in_range() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    seed_weekday_work_schedule_for_user(&pool, employee.id, "follow_weekly_pattern").await;
    seed_annual_balance(&pool, &employee, 5).await;

    // 2026-07-11 (Sat) - 2026-07-12 (Sun): 稼働日が 0 日の申請は、残高があっても
    // H-1 の Consumption Target Days 決定により拒否される。
    let (status, body) = request_json(
        router(pool, employee),
        "POST",
        "/api/requests/leave",
        Some(json!({
            "leave_type": "annual",
            "start_date": "2026-07-11",
            "end_date": "2026-07-12",
            "reason": "vacation"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["code"], "LEAVE_REQUEST_NO_WORKING_DAYS");
}

#[tokio::test]
async fn annual_leave_request_rejects_when_work_schedule_is_unresolved() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");
    // 勤務体系を一切割り当てないユーザー。resolved workday を解決できない日を
    // 含む場合は、暦日フォールバックせず fail-closed でエラーにする。
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    seed_annual_balance(&pool, &employee, 5).await;

    let (status, body) = request_json(
        router(pool, employee),
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

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["code"], "LEAVE_REQUEST_SCHEDULE_UNRESOLVED");
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
    seed_weekday_work_schedule_for_user(&pool, employee.id, "follow_weekly_pattern").await;
    seed_annual_balance(&pool, &employee, 5).await;

    // 2026-07-10 (Fri) - 2026-07-13 (Mon): 4 暦日だが稼働日は金・月の 2 日のみ。
    // H-1 修正前は暦日ベースで 4 * 480 = 1920 分を誤って消費していた。
    let (status, body) = request_json(
        router(pool.clone(), employee.clone()),
        "POST",
        "/api/requests/leave",
        Some(json!({
            "leave_type": "annual",
            "start_date": "2026-07-10",
            "end_date": "2026-07-13",
            "reason": "vacation"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let request_id = body["id"].as_str().expect("request id").to_string();

    assert_eq!(
        balance_minutes(pool.clone(), employee.clone(), "2026-07-13").await,
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
        balance_minutes(pool.clone(), employee.clone(), "2026-07-13").await,
        1440,
        "only the 2 working days (Fri + Mon) should be consumed: 2400 - 960 = 1440"
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
        balance_minutes(pool.clone(), employee, "2026-07-13").await,
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
