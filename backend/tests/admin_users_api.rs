use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::get,
    Extension, Router,
};
use serde_json::json;
use sqlx::PgPool;
use timekeeper_backend::{
    handlers::admin::users,
    models::user::{CreateUser, UpdateUser, User, UserRole},
    state::AppState,
    types::UserId,
};
use tower::ServiceExt;

mod support;

use support::{
    create_test_token, seed_user, test_config, test_pool,
};

async fn integration_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static GUARD: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();
    GUARD.get_or_init(|| tokio::sync::Mutex::new(())).lock().await
}

fn test_router_with_state(pool: PgPool, user: User) -> Router {
    let state = AppState::new(pool, None, None, None, test_config());
    Router::new()
        .route("/api/admin/users", get(users::get_users).post(users::create_user))
        .route("/api/admin/users/{id}", get(users::update_user).delete(users::delete_user))
        .layer(Extension(user))
        .with_state(state)
}

async fn get_users_list(pool: &PgPool, user: &User) -> StatusCode {
    let token = create_test_token(user.id, user.role.clone());
    let app = test_router_with_state(pool.clone(), user.clone());
    
    let request = Request::builder()
        .uri("/api/admin/users")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    response.status()
}

#[tokio::test]
async fn test_admin_can_list_all_users() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let _regular = seed_user(&pool, UserRole::Employee, false).await;
    
    let status = get_users_list(&pool, &admin).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_non_admin_cannot_list_users() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    
    let status = get_users_list(&pool, &employee).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_system_admin_can_create_user() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;
    
    let new_user = CreateUser {
        username: "newemployee".to_string(),
        password: "SecureP@ssw0rd123".to_string(),
        full_name: "New Employee".to_string(),
        email: "new.employee@example.com".to_string(),
        role: UserRole::Employee,
        is_system_admin: false,
    };
    
    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let state = AppState::new(pool.clone(), None, None, None, test_config());
    let app = Router::new()
        .route("/api/admin/users", axum::routing::post(users::create_user))
        .layer(Extension(sysadmin))
        .with_state(state);
    
    let request = Request::builder()
        .method("POST")
        .uri("/api/admin/users")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!(new_user).to_string()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_regular_admin_cannot_create_user() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Admin, false).await;
    
    let new_user = CreateUser {
        username: "newemployee".to_string(),
        password: "SecureP@ssw0rd123".to_string(),
        full_name: "New Employee".to_string(),
        email: "new.employee@example.com".to_string(),
        role: UserRole::Employee,
        is_system_admin: false,
    };
    
    let token = create_test_token(admin.id, admin.role.clone());
    let state = AppState::new(pool.clone(), None, None, None, test_config());
    let app = Router::new()
        .route("/api/admin/users", axum::routing::post(users::create_user))
        .layer(Extension(admin))
        .with_state(state);
    
    let request = Request::builder()
        .method("POST")
        .uri("/api/admin/users")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!(new_user).to_string()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_cannot_create_user_with_duplicate_username() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let existing = seed_user(&pool, UserRole::Employee, false).await;
    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;
    
    let new_user = CreateUser {
        username: existing.username.clone(),
        password: "SecureP@ssw0rd123".to_string(),
        full_name: "Another User".to_string(),
        email: "another@example.com".to_string(),
        role: UserRole::Employee,
        is_system_admin: false,
    };
    
    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let state = AppState::new(pool.clone(), None, None, None, test_config());
    let app = Router::new()
        .route("/api/admin/users", axum::routing::post(users::create_user))
        .layer(Extension(sysadmin))
        .with_state(state);
    
    let request = Request::builder()
        .method("POST")
        .uri("/api/admin/users")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!(new_user).to_string()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_system_admin_can_update_user() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;
    let target = seed_user(&pool, UserRole::Employee, false).await;
    
    let update = UpdateUser {
        full_name: Some("Updated Name".to_string()),
        email: None,
        role: None,
        is_system_admin: None,
    };
    
    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let state = AppState::new(pool.clone(), None, None, None, test_config());
    let app = Router::new()
        .route("/api/admin/users/{id}", axum::routing::put(users::update_user))
        .layer(Extension(sysadmin))
        .with_state(state);
    
    let request = Request::builder()
        .method("PUT")
        .uri(format!("/api/admin/users/{}", target.id))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!(update).to_string()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_cannot_update_with_duplicate_email() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;
    let user1 = seed_user(&pool, UserRole::Employee, false).await;
    let user2 = seed_user(&pool, UserRole::Employee, false).await;
    
    let update = UpdateUser {
        full_name: None,
        email: Some(user1.email.clone()),
        role: None,
        is_system_admin: None,
    };
    
    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let state = AppState::new(pool.clone(), None, None, None, test_config());
    let app = Router::new()
        .route("/api/admin/users/{id}", axum::routing::put(users::update_user))
        .layer(Extension(sysadmin))
        .with_state(state);
    
    let request = Request::builder()
        .method("PUT")
        .uri(format!("/api/admin/users/{}", user2.id))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!(update).to_string()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_cannot_delete_self() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;
    
    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let state = AppState::new(pool.clone(), None, None, None, test_config());
    let app = Router::new()
        .route("/api/admin/users/{id}", axum::routing::delete(users::delete_user))
        .layer(Extension(sysadmin.clone()))
        .with_state(state);
    
    let request = Request::builder()
        .method("DELETE")
        .uri(format!("/api/admin/users/{}?hard=false", sysadmin.id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_regular_admin_cannot_reset_mfa() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let _target = seed_user(&pool, UserRole::Employee, false).await;
    
    let token = create_test_token(admin.id, admin.role.clone());
    let state = AppState::new(pool.clone(), None, None, None, test_config());
    let app = Router::new()
        .route("/api/admin/users/{id}/reset-mfa", axum::routing::post(users::reset_user_mfa))
        .layer(Extension(admin))
        .with_state(state);
    
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/admin/users/{}/reset-mfa", UserId::new()))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({"user_id": UserId::new().to_string()}).to_string()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
