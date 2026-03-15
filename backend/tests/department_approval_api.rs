//! Integration tests for department-scoped approval authorization.
//!
//! Scenarios tested:
//! ✅ Manager approves direct department member's request
//! ✅ Manager approves subordinate (3-level deep) department member's request
//! ✅ system_admin approves any member's request (backward compat)
//! ❌ Manager cannot approve member of unrelated department (403)
//! ❌ Manager cannot approve their own request (403)
//! ❌ Manager cannot approve sibling department member (403)
//! ❌ Employee cannot approve any request (403)

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Extension, Router,
};
use serde_json::json;
use sqlx::PgPool;
use timekeeper_backend::{
    handlers::{admin::requests as admin_requests, requests as user_requests},
    models::user::{User, UserRole},
    state::AppState,
    types::DepartmentId,
};
use tower::ServiceExt;

mod support;

use support::{create_test_token, seed_leave_request, seed_user, test_config, test_pool};

async fn integration_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static GUARD: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();
    GUARD
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

// ─── router helpers ──────────────────────────────────────────────────────────

fn approval_router(pool: PgPool, actor: User) -> Router {
    let state = AppState::new(pool, None, None, None, test_config());
    Router::new()
        .route(
            "/api/admin/requests/{id}/approve",
            axum::routing::put(admin_requests::approve_request),
        )
        .route(
            "/api/admin/requests/{id}/reject",
            axum::routing::put(admin_requests::reject_request),
        )
        .layer(Extension(actor))
        .with_state(state)
}

fn leave_request_router(pool: PgPool, user: User) -> Router {
    let state = AppState::new(pool, None, None, None, test_config());
    Router::new()
        .route(
            "/api/requests/leave",
            axum::routing::post(user_requests::create_leave_request),
        )
        .layer(Extension(user))
        .with_state(state)
}

// ─── DB helpers ──────────────────────────────────────────────────────────────

async fn create_dept(pool: &PgPool, name: &str, parent_id: Option<&str>) -> String {
    let id = DepartmentId::new().to_string();
    sqlx::query("INSERT INTO departments (id, name, parent_id) VALUES ($1, $2, $3)")
        .bind(&id)
        .bind(name)
        .bind(parent_id)
        .execute(pool)
        .await
        .expect("insert department");
    id
}

async fn assign_user_to_dept(pool: &PgPool, user_id: &str, dept_id: &str) {
    sqlx::query("UPDATE users SET department_id = $1 WHERE id = $2")
        .bind(dept_id)
        .bind(user_id)
        .execute(pool)
        .await
        .expect("assign user to dept");
}

async fn assign_manager_to_dept(pool: &PgPool, user_id: &str, dept_id: &str) {
    sqlx::query(
        "INSERT INTO department_managers (department_id, user_id) VALUES ($1, $2) \
         ON CONFLICT DO NOTHING",
    )
    .bind(dept_id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("assign manager to dept");
}

async fn submit_leave_request(pool: PgPool, employee: User) -> String {
    let token = create_test_token(employee.id, employee.role.clone());
    let app = leave_request_router(pool, employee);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/requests/leave")
                .header("Authorization", format!("Bearer {}", token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "leave_type": "annual",
                        "start_date": "2025-06-01",
                        "end_date": "2025-06-03",
                        "reason": "vacation"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    json["id"].as_str().expect("request id").to_string()
}

// ─── tests ───────────────────────────────────────────────────────────────────

/// Manager in the same department as employee can approve.
#[tokio::test]
async fn test_manager_can_approve_direct_dept_member() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let manager = seed_user(&pool, UserRole::Manager, false).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let dept_id = create_dept(&pool, "TeamA", None).await;
    assign_user_to_dept(&pool, &employee.id.to_string(), &dept_id).await;
    assign_manager_to_dept(&pool, &manager.id.to_string(), &dept_id).await;

    let request_id = submit_leave_request(pool.clone(), employee).await;

    let token = create_test_token(manager.id, manager.role.clone());
    let app = approval_router(pool, manager);

    let resp = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/admin/requests/{}/approve", request_id))
                .header("Authorization", format!("Bearer {}", token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"comment": "Approved!"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

/// Manager can approve member of subordinate (child) department.
#[tokio::test]
async fn test_manager_can_approve_subordinate_dept_member() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let manager = seed_user(&pool, UserRole::Manager, false).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;

    // dept_a → dept_b → dept_c (3 levels)
    let dept_a = create_dept(&pool, "BU-Alpha", None).await;
    let dept_b = create_dept(&pool, "Division-Beta", Some(&dept_a)).await;
    let dept_c = create_dept(&pool, "Team-Gamma", Some(&dept_b)).await;

    // Manager is assigned to dept_a (top), employee is in dept_c (leaf)
    assign_manager_to_dept(&pool, &manager.id.to_string(), &dept_a).await;
    assign_user_to_dept(&pool, &employee.id.to_string(), &dept_c).await;

    let request_id = submit_leave_request(pool.clone(), employee).await;

    let token = create_test_token(manager.id, manager.role.clone());
    let app = approval_router(pool, manager);

    let resp = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/admin/requests/{}/approve", request_id))
                .header("Authorization", format!("Bearer {}", token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"comment": "Deep approve"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

/// system_admin can approve any member regardless of department.
#[tokio::test]
async fn test_system_admin_can_approve_any_member() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let sysadmin = seed_user(&pool, UserRole::Manager, true).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    // employee has no department — sysadmin should still be able to approve

    let request_id = submit_leave_request(pool.clone(), employee).await;

    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let app = approval_router(pool, sysadmin);

    let resp = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/admin/requests/{}/approve", request_id))
                .header("Authorization", format!("Bearer {}", token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({"comment": "Sysadmin approves"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

/// Manager cannot approve member of an unrelated department.
#[tokio::test]
async fn test_manager_cannot_approve_unrelated_dept_member() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let manager = seed_user(&pool, UserRole::Manager, false).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let dept_manager = create_dept(&pool, "Manager-Dept", None).await;
    let dept_other = create_dept(&pool, "Other-Dept", None).await;

    assign_manager_to_dept(&pool, &manager.id.to_string(), &dept_manager).await;
    assign_user_to_dept(&pool, &employee.id.to_string(), &dept_other).await;

    let request_id = submit_leave_request(pool.clone(), employee).await;

    let token = create_test_token(manager.id, manager.role.clone());
    let app = approval_router(pool, manager);

    let resp = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/admin/requests/{}/approve", request_id))
                .header("Authorization", format!("Bearer {}", token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"comment": "Should fail"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

/// Manager cannot approve member of a sibling department (same parent, different child).
#[tokio::test]
async fn test_manager_cannot_approve_sibling_dept_member() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let manager = seed_user(&pool, UserRole::Manager, false).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let parent_dept = create_dept(&pool, "Parent", None).await;
    let dept_manager = create_dept(&pool, "Sibling-A", Some(&parent_dept)).await;
    let dept_sibling = create_dept(&pool, "Sibling-B", Some(&parent_dept)).await;

    assign_manager_to_dept(&pool, &manager.id.to_string(), &dept_manager).await;
    assign_user_to_dept(&pool, &employee.id.to_string(), &dept_sibling).await;

    let request_id = submit_leave_request(pool.clone(), employee).await;

    let token = create_test_token(manager.id, manager.role.clone());
    let app = approval_router(pool, manager);

    let resp = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/admin/requests/{}/approve", request_id))
                .header("Authorization", format!("Bearer {}", token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"comment": "Should fail"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

/// Manager cannot approve their own request.
#[tokio::test]
async fn test_manager_cannot_approve_own_request() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let manager = seed_user(&pool, UserRole::Manager, false).await;

    let dept_id = create_dept(&pool, "Self-Dept", None).await;
    assign_user_to_dept(&pool, &manager.id.to_string(), &dept_id).await;
    assign_manager_to_dept(&pool, &manager.id.to_string(), &dept_id).await;

    let request_id = submit_leave_request(pool.clone(), manager.clone()).await;

    let token = create_test_token(manager.id, manager.role.clone());
    let app = approval_router(pool, manager);

    let resp = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/admin/requests/{}/approve", request_id))
                .header("Authorization", format!("Bearer {}", token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"comment": "self-approve"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

/// Employee cannot approve any request.
#[tokio::test]
async fn test_employee_cannot_approve_request() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let other = seed_user(&pool, UserRole::Employee, false).await;

    let dept_id = create_dept(&pool, "Employee-Dept", None).await;
    assign_user_to_dept(&pool, &other.id.to_string(), &dept_id).await;

    let leave = seed_leave_request(
        &pool,
        other.id,
        timekeeper_backend::models::leave_request::LeaveType::Annual,
        chrono::NaiveDate::from_ymd_opt(2025, 7, 1).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2025, 7, 3).unwrap(),
    )
    .await;

    let token = create_test_token(employee.id, employee.role.clone());
    let app = approval_router(pool, employee);

    let resp = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/admin/requests/{}/approve", leave.id))
                .header("Authorization", format!("Bearer {}", token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"comment": "I approve"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

/// Manager can reject as well as approve a direct member's request.
#[tokio::test]
async fn test_manager_can_reject_direct_dept_member() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let manager = seed_user(&pool, UserRole::Manager, false).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let dept_id = create_dept(&pool, "RejectDept", None).await;
    assign_user_to_dept(&pool, &employee.id.to_string(), &dept_id).await;
    assign_manager_to_dept(&pool, &manager.id.to_string(), &dept_id).await;

    let request_id = submit_leave_request(pool.clone(), employee).await;

    let token = create_test_token(manager.id, manager.role.clone());
    let app = approval_router(pool, manager);

    let resp = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/admin/requests/{}/reject", request_id))
                .header("Authorization", format!("Bearer {}", token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({"comment": "Not approved this time"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}
