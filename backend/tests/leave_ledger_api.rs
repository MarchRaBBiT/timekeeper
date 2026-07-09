use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::{get, post, put},
    Extension, Router,
};
use chrono::NaiveDate;
use serde_json::{json, Value};
use sqlx::PgPool;
use timekeeper_backend::{
    handlers::{admin, leave_ledger},
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
        .route(
            "/api/leave-balances/me",
            get(leave_ledger::get_my_leave_balance),
        )
        .route(
            "/api/admin/users/{user_id}/leave-balances",
            get(admin::get_user_leave_balance),
        )
        .route("/api/admin/leave-grants/run", post(admin::run_leave_grants))
        .route(
            "/api/admin/leave-ledger/adjust",
            post(admin::adjust_leave_ledger),
        )
        .route(
            "/api/admin/users/{user_id}/hire-date",
            put(admin::set_user_hire_date),
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

#[tokio::test]
async fn grant_run_writes_ledger_and_self_balance_reads_it() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let hire_path = format!("/api/admin/users/{}/hire-date", employee.id);
    let (status, body) = request_json(
        router(pool.clone(), admin.clone()),
        "PUT",
        &hire_path,
        Some(json!({ "hire_date": "2026-01-01" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["hire_date"], "2026-01-01");

    let (status, body) = request_json(
        router(pool.clone(), admin),
        "POST",
        "/api/admin/leave-grants/run",
        Some(json!({
            "base_date": "2026-07-01",
            "dry_run": false,
            "user_ids": [employee.id.to_string()]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["granted"].as_array().expect("grants").len(), 1);
    assert_eq!(body["granted"][0]["granted_minutes"], 4800);
    assert_eq!(body["granted"][0]["granted_days"], 10);
    assert!(body["granted"][0]["lot_id"].as_str().is_some());

    let (status, body) = request_json(
        router(pool, employee),
        "GET",
        "/api/leave-balances/me?as_of=2026-07-05",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["available_minutes"], 4800);
    assert_eq!(body["available_days"], 10.0);
    assert_eq!(body["upcoming_expiries"][0]["expires_at"], "2028-07-01");
    assert_eq!(body["obligations"][0]["required_minutes"], 2400);
    assert_eq!(body["obligations"][0]["status"], "ok");
}

/// M-1a: 付与バッチ実行中（leave_grant_batch_lock が claim 済み）は 409 を返し、
/// stale な claim（クラッシュ残留）は自動で奪い直して実行できること。
/// dry_run は排他対象外で、claim 中でも実行できること。
#[tokio::test]
async fn grant_run_is_rejected_while_batch_lock_is_claimed_and_reclaims_stale_lock() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let hire_path = format!("/api/admin/users/{}/hire-date", employee.id);
    let (status, body) = request_json(
        router(pool.clone(), admin.clone()),
        "PUT",
        &hire_path,
        Some(json!({ "hire_date": "2026-01-01" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    // 別バッチが実行中の想定で claim を立てる。
    sqlx::query(
        "UPDATE leave_grant_batch_lock
         SET running = TRUE, claim_token = $1, started_at = NOW(), started_by = 'other-run'
         WHERE id = 1",
    )
    .bind(Uuid::new_v4())
    .execute(&pool)
    .await
    .expect("claim batch lock");

    let run_payload = json!({
        "base_date": "2026-07-01",
        "dry_run": false,
        "user_ids": [employee.id.to_string()]
    });
    let (status, body) = request_json(
        router(pool.clone(), admin.clone()),
        "POST",
        "/api/admin/leave-grants/run",
        Some(run_payload.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["code"], "CONFLICT");

    // dry_run は排他対象外なので claim 中でもプレビューできる。
    let mut dry_run_payload = run_payload.clone();
    dry_run_payload["dry_run"] = json!(true);
    let (status, body) = request_json(
        router(pool.clone(), admin.clone()),
        "POST",
        "/api/admin/leave-grants/run",
        Some(dry_run_payload),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["dry_run"], true);

    // stale な claim（クラッシュ残留想定）は奪い直して実行できる。
    sqlx::query(
        "UPDATE leave_grant_batch_lock SET started_at = NOW() - INTERVAL '1 hour' WHERE id = 1",
    )
    .execute(&pool)
    .await
    .expect("make claim stale");

    let (status, body) = request_json(
        router(pool.clone(), admin),
        "POST",
        "/api/admin/leave-grants/run",
        Some(run_payload),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["granted"].as_array().expect("grants").len(), 1);

    // 実行完了後は claim が解放されている。
    let running: bool =
        sqlx::query_scalar("SELECT running FROM leave_grant_batch_lock WHERE id = 1")
            .fetch_one(&pool)
            .await
            .expect("read lock row");
    assert!(!running, "batch lock must be released after the run");
}

#[tokio::test]
async fn adjust_api_supports_dry_run_and_initial_balance_commit() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let payload = json!({
        "user_id": employee.id.to_string(),
        "amount_minutes": 2400,
        "day_equivalent_minutes": 480,
        "granted_at": "2025-10-01",
        "expires_at": "2027-10-01",
        "grant_base_date": "2025-10-01",
        "reason": "initial migration",
        "dry_run": true
    });
    let (status, body) = request_json(
        router(pool.clone(), admin.clone()),
        "POST",
        "/api/admin/leave-ledger/adjust",
        Some(payload.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["dry_run"], true);
    assert!(body["entry"].is_null());
    assert_eq!(body["balance_after_minutes"], 2400);

    let mut commit_payload = payload;
    commit_payload["dry_run"] = json!(false);
    let (status, body) = request_json(
        router(pool.clone(), admin),
        "POST",
        "/api/admin/leave-ledger/adjust",
        Some(commit_payload),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["dry_run"], false);
    assert_eq!(body["entry"]["kind"], "adjust");
    assert_eq!(body["balance_after_days"], 5.0);

    let (status, body) = request_json(
        router(pool, employee),
        "GET",
        "/api/leave-balances/me?as_of=2027-09-30",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["available_minutes"], 2400);
}

#[tokio::test]
async fn admin_balance_read_is_limited_to_manager_department_scope() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");
    let scoped_manager = seed_user(&pool, UserRole::Manager, false).await;
    let other_manager = seed_user(&pool, UserRole::Manager, false).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let department_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO departments (id, name) VALUES ($1, $2)")
        .bind(&department_id)
        .bind(format!("Leave Ledger Dept {department_id}"))
        .execute(&pool)
        .await
        .expect("insert department");
    sqlx::query("INSERT INTO department_managers (department_id, user_id) VALUES ($1, $2)")
        .bind(&department_id)
        .bind(scoped_manager.id.to_string())
        .execute(&pool)
        .await
        .expect("assign manager");
    sqlx::query("UPDATE users SET department_id = $1 WHERE id = $2")
        .bind(&department_id)
        .bind(employee.id.to_string())
        .execute(&pool)
        .await
        .expect("assign employee department");

    let path = format!(
        "/api/admin/users/{}/leave-balances?as_of=2026-07-05",
        employee.id
    );
    let (status, body) =
        request_json(router(pool.clone(), scoped_manager), "GET", &path, None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["user_id"], employee.id.to_string());

    let (status, body) = request_json(router(pool, other_manager), "GET", &path, None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "FORBIDDEN");
}

/// M-5: migration 056 が追加した `leave_ledger_adjust_new_lot_fields` CHECK。
/// kind='adjust' で `granted_at` を設定する（= 新規ロット投入のつもり）のに
/// `expires_at` を欠かす行は DB レベルで拒否されるべき。
#[tokio::test]
async fn adjust_new_lot_requires_expires_at_at_the_database_level() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let granted_at = NaiveDate::from_ymd_opt(2026, 1, 1).expect("date");

    let result = sqlx::query(
        "INSERT INTO leave_ledger_entries (
            id, user_id, leave_type, kind, lot_id, amount_minutes,
            day_equivalent_minutes, granted_at, expires_at, grant_base_date,
            reason, effective_at
         ) VALUES ($1,$2,'annual','adjust',$3,2400,480,$4,NULL,NULL,'missing expiry',$4)",
    )
    .bind(Uuid::new_v4())
    .bind(employee.id.to_string())
    .bind(Uuid::new_v4())
    .bind(granted_at)
    .execute(&pool)
    .await;

    let error = result.expect_err("DB must reject adjust rows missing expires_at");
    let message = error.to_string();
    assert!(
        message.contains("leave_ledger_adjust_new_lot_fields"),
        "expected the new-lot CHECK violation, got: {message}"
    );

    // 対照: 既存ロットへの調整（granted_at が NULL）は expires_at が NULL でも許可される。
    sqlx::query(
        "INSERT INTO leave_ledger_entries (
            id, user_id, leave_type, kind, lot_id, amount_minutes,
            day_equivalent_minutes, granted_at, expires_at, grant_base_date,
            reason, effective_at
         ) VALUES ($1,$2,'annual','adjust',$3,-480,480,NULL,NULL,NULL,'existing lot adjust',$4)",
    )
    .bind(Uuid::new_v4())
    .bind(employee.id.to_string())
    .bind(Uuid::new_v4())
    .bind(granted_at)
    .execute(&pool)
    .await
    .expect("existing-lot adjust without granted_at must be allowed");
}
