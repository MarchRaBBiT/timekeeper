use axum::{
    body::Body,
    http::{Request, StatusCode},
    Extension, Router,
};
use serde_json::json;
use sqlx::PgPool;
use timekeeper_backend::{
    handlers::auth,
    models::user::{User, UserRole},
    state::AppState,
    utils::password::hash_password,
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
        .route("/api/auth/password", axum::routing::put(auth::change_password))
        .layer(Extension(user))
        .with_state(state)
}

const TEST_ORIGIN: &str = "http://localhost:8000";

#[tokio::test]
async fn test_change_password_with_correct_current_password_succeeds() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let password = "CurrentPass123!";
    let mut employee = seed_user(&pool, UserRole::Employee, false).await;
    let password_hash = hash_password(password).expect("hash password");
    employee.password_hash = password_hash;
    
    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());
    
    let payload = json!({
        "current_password": password,
        "new_password": "NewPass456!@#"
    });
    let request = Request::builder()
        .method("PUT")
        .uri("/api/auth/password")
        .header("Authorization", format!("Bearer {}", token))
        .header("Origin", TEST_ORIGIN)
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_change_password_with_incorrect_current_password_fails() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let password = "CurrentPass123!";
    let mut employee = seed_user(&pool, UserRole::Employee, false).await;
    let password_hash = hash_password(password).expect("hash password");
    employee.password_hash = password_hash;
    
    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());
    
    let payload = json!({
        "current_password": "WrongPass123!",
        "new_password": "NewPass456!@#"
    });
    let request = Request::builder()
        .method("PUT")
        .uri("/api/auth/password")
        .header("Authorization", format!("Bearer {}", token))
        .header("Origin", TEST_ORIGIN)
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_change_password_same_as_current_fails() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let password = "CurrentPass123!";
    let mut employee = seed_user(&pool, UserRole::Employee, false).await;
    let password_hash = hash_password(password).expect("hash password");
    employee.password_hash = password_hash;
    
    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());
    
    let payload = json!({
        "current_password": password,
        "new_password": password
    });
    let request = Request::builder()
        .method("PUT")
        .uri("/api/auth/password")
        .header("Authorization", format!("Bearer {}", token))
        .header("Origin", TEST_ORIGIN)
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_change_password_with_weak_password_fails() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let password = "CurrentPass123!";
    let mut employee = seed_user(&pool, UserRole::Employee, false).await;
    let password_hash = hash_password(password).expect("hash password");
    employee.password_hash = password_hash;
    
    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());
    
    let payload = json!({
        "current_password": password,
        "new_password": "weak"
    });
    let request = Request::builder()
        .method("PUT")
        .uri("/api/auth/password")
        .header("Authorization", format!("Bearer {}", token))
        .header("Origin", TEST_ORIGIN)
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_change_password_with_short_password_fails() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let password = "CurrentPass123!";
    let mut employee = seed_user(&pool, UserRole::Employee, false).await;
    let password_hash = hash_password(password).expect("hash password");
    employee.password_hash = password_hash;
    
    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());
    
    let payload = json!({
        "current_password": password,
        "new_password": "Short1!"
    });
    let request = Request::builder()
        .method("PUT")
        .uri("/api/auth/password")
        .header("Authorization", format!("Bearer {}", token))
        .header("Origin", TEST_ORIGIN)
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_change_password_without_uppercase_fails() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let password = "CurrentPass123!";
    let mut employee = seed_user(&pool, UserRole::Employee, false).await;
    let password_hash = hash_password(password).expect("hash password");
    employee.password_hash = password_hash;
    
    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());
    
    let payload = json!({
        "current_password": password,
        "new_password": "lowercase123!"
    });
    let request = Request::builder()
        .method("PUT")
        .uri("/api/auth/password")
        .header("Authorization", format!("Bearer {}", token))
        .header("Origin", TEST_ORIGIN)
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_change_password_without_number_fails() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let password = "CurrentPass123!";
    let mut employee = seed_user(&pool, UserRole::Employee, false).await;
    let password_hash = hash_password(password).expect("hash password");
    employee.password_hash = password_hash;
    
    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());
    
    let payload = json!({
        "current_password": password,
        "new_password": "NoNumbersHere!"
    });
    let request = Request::builder()
        .method("PUT")
        .uri("/api/auth/password")
        .header("Authorization", format!("Bearer {}", token))
        .header("Origin", TEST_ORIGIN)
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_change_password_without_special_char_fails() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let password = "CurrentPass123!";
    let mut employee = seed_user(&pool, UserRole::Employee, false).await;
    let password_hash = hash_password(password).expect("hash password");
    employee.password_hash = password_hash;
    
    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());
    
    let payload = json!({
        "current_password": password,
        "new_password": "NoSpecialChar123"
    });
    let request = Request::builder()
        .method("PUT")
        .uri("/api/auth/password")
        .header("Authorization", format!("Bearer {}", token))
        .header("Origin", TEST_ORIGIN)
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_change_password_empty_current_password_fails() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;
    
    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());
    
    let payload = json!({
        "current_password": "",
        "new_password": "NewPass456!@#"
    });
    let request = Request::builder()
        .method("PUT")
        .uri("/api/auth/password")
        .header("Authorization", format!("Bearer {}", token))
        .header("Origin", TEST_ORIGIN)
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
