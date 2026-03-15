use axum::{
    body::Body,
    http::{Request, StatusCode},
    Extension, Router,
};
use serde_json::json;
use sqlx::PgPool;
use timekeeper_backend::{
    handlers::admin::departments as dept_handlers,
    models::user::{User, UserRole},
    state::AppState,
    types::DepartmentId,
};
use tower::ServiceExt;

mod support;

use support::{create_test_token, seed_user, test_config, test_pool};

async fn integration_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static GUARD: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();
    GUARD
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

fn manager_read_router(pool: PgPool, user: User) -> Router {
    let state = AppState::new(pool, None, None, None, test_config());
    Router::new()
        .route(
            "/api/admin/departments",
            axum::routing::get(dept_handlers::list_departments),
        )
        .route(
            "/api/admin/departments/{id}",
            axum::routing::get(dept_handlers::get_department),
        )
        .route(
            "/api/admin/departments/{id}/managers",
            axum::routing::get(dept_handlers::list_department_managers_handler),
        )
        .layer(Extension(user))
        .with_state(state)
}

fn sysadmin_write_router(pool: PgPool, user: User) -> Router {
    let state = AppState::new(pool, None, None, None, test_config());
    Router::new()
        .route(
            "/api/admin/departments",
            axum::routing::post(dept_handlers::create_department),
        )
        .route(
            "/api/admin/departments/{id}",
            axum::routing::put(dept_handlers::update_department)
                .delete(dept_handlers::delete_department),
        )
        .route(
            "/api/admin/departments/{id}/managers",
            axum::routing::post(dept_handlers::assign_manager_handler),
        )
        .route(
            "/api/admin/departments/{id}/managers/{uid}",
            axum::routing::delete(dept_handlers::remove_manager_handler),
        )
        .layer(Extension(user))
        .with_state(state)
}

// ─── helpers ────────────────────────────────────────────────────────────────

async fn create_department_in_db(pool: &PgPool, name: &str) -> String {
    let id = DepartmentId::new().to_string();
    sqlx::query("INSERT INTO departments (id, name) VALUES ($1, $2)")
        .bind(&id)
        .bind(name)
        .execute(pool)
        .await
        .expect("insert department");
    id
}

// ─── tests: listing ──────────────────────────────────────────────────────────

#[tokio::test]
async fn test_manager_can_list_departments() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let manager = seed_user(&pool, UserRole::Manager, false).await;
    create_department_in_db(&pool, "Engineering").await;

    let token = create_test_token(manager.id, manager.role.clone());
    let app = manager_read_router(pool, manager);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/admin/departments")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.as_array().is_some());
    assert!(!json.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_employee_cannot_list_departments() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let token = create_test_token(employee.id, employee.role.clone());
    let app = manager_read_router(pool, employee);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/admin/departments")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// ─── tests: single department detail ─────────────────────────────────────────

#[tokio::test]
async fn test_manager_can_get_department_by_id() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let manager = seed_user(&pool, UserRole::Manager, false).await;
    let dept_id = create_department_in_db(&pool, "HR").await;

    let token = create_test_token(manager.id, manager.role.clone());
    let app = manager_read_router(pool, manager);

    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/admin/departments/{}", dept_id))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "HR");
    assert_eq!(json["id"], dept_id);
}

#[tokio::test]
async fn test_get_department_returns_404_for_missing() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let manager = seed_user(&pool, UserRole::Manager, false).await;
    let token = create_test_token(manager.id, manager.role.clone());
    let app = manager_read_router(pool, manager);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/admin/departments/nonexistent-id")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ─── tests: create ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_system_admin_can_create_department() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let sysadmin = seed_user(&pool, UserRole::Manager, true).await;
    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let app = sysadmin_write_router(pool, sysadmin);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/departments")
                .header("Authorization", format!("Bearer {}", token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"name": "Finance"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "Finance");
    assert!(json["id"].is_string());
}

#[tokio::test]
async fn test_non_system_admin_cannot_create_department() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let manager = seed_user(&pool, UserRole::Manager, false).await;
    let token = create_test_token(manager.id, manager.role.clone());
    let app = sysadmin_write_router(pool, manager);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/departments")
                .header("Authorization", format!("Bearer {}", token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"name": "Forbidden"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_create_department_with_empty_name_fails() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let sysadmin = seed_user(&pool, UserRole::Manager, true).await;
    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let app = sysadmin_write_router(pool, sysadmin);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/departments")
                .header("Authorization", format!("Bearer {}", token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"name": "  "}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ─── tests: update ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_system_admin_can_update_department() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let sysadmin = seed_user(&pool, UserRole::Manager, true).await;
    let dept_id = create_department_in_db(&pool, "Old Name").await;

    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let app = sysadmin_write_router(pool, sysadmin);

    let resp = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/admin/departments/{}", dept_id))
                .header("Authorization", format!("Bearer {}", token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"name": "New Name"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "New Name");
}

// ─── tests: delete ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_system_admin_can_delete_department_without_children() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let sysadmin = seed_user(&pool, UserRole::Manager, true).await;
    let dept_id = create_department_in_db(&pool, "Temp Dept").await;

    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let app = sysadmin_write_router(pool, sysadmin);

    let resp = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/admin/departments/{}", dept_id))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_delete_department_with_children_fails() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let sysadmin = seed_user(&pool, UserRole::Manager, true).await;
    let parent_id = create_department_in_db(&pool, "Parent Dept").await;

    // create a child department
    let child_id = DepartmentId::new().to_string();
    sqlx::query("INSERT INTO departments (id, name, parent_id) VALUES ($1, $2, $3)")
        .bind(&child_id)
        .bind("Child Dept")
        .bind(&parent_id)
        .execute(&pool)
        .await
        .expect("insert child dept");

    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let app = sysadmin_write_router(pool, sysadmin);

    let resp = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/admin/departments/{}", parent_id))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

// ─── tests: manager assignment ───────────────────────────────────────────────

#[tokio::test]
async fn test_system_admin_can_assign_and_remove_manager() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let sysadmin = seed_user(&pool, UserRole::Manager, true).await;
    let manager_user = seed_user(&pool, UserRole::Manager, false).await;
    let dept_id = create_department_in_db(&pool, "Ops").await;

    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let app = sysadmin_write_router(pool, sysadmin);

    // Assign
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/admin/departments/{}/managers", dept_id))
                .header("Authorization", format!("Bearer {}", token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({"user_id": manager_user.id.to_string()}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    // Remove
    let resp = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!(
                    "/api/admin/departments/{}/managers/{}",
                    dept_id, manager_user.id
                ))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}
