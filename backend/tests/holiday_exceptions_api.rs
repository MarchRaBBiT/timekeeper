use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::Response,
    Extension, Router,
};
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use std::sync::Once;
use std::time::Duration;
use timekeeper_backend::{
    handlers::holiday_exceptions,
    models::user::{User, UserRole},
    services::holiday_exception::{HolidayExceptionService, HolidayExceptionServiceTrait},
    state::AppState,
};
use tower::ServiceExt;

mod support;

use support::{create_test_token, seed_user, test_config, test_pool};

async fn integration_guard() -> tokio::sync::MutexGuard<'static, ()> {
    init_tracing();
    static GUARD: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();
    GUARD
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

async fn setup_test_pool() -> PgPool {
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    pool
}

fn init_tracing() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_test_writer()
            .init();
    });
}

async fn oneshot_with_retry<F>(app: &Router, build_request: F) -> Response
where
    F: Fn() -> Request<Body>,
{
    let mut response = app.clone().oneshot(build_request()).await.unwrap();
    for _ in 0..2 {
        if response.status() != StatusCode::INTERNAL_SERVER_ERROR {
            return response;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
        response = app.clone().oneshot(build_request()).await.unwrap();
    }
    response
}

fn test_router_with_state(pool: PgPool, user: User) -> Router {
    let state = AppState::new(pool.clone(), None, None, None, test_config());
    let holiday_exception_service: Arc<dyn HolidayExceptionServiceTrait> =
        Arc::new(HolidayExceptionService::new(pool));

    Router::new()
        .route(
            "/api/admin/users/{user_id}/holiday-exceptions",
            axum::routing::post(holiday_exceptions::create_holiday_exception),
        )
        .route(
            "/api/admin/users/{user_id}/holiday-exceptions",
            axum::routing::get(holiday_exceptions::list_holiday_exceptions),
        )
        .route(
            "/api/admin/users/{user_id}/holiday-exceptions/{id}",
            axum::routing::delete(holiday_exceptions::delete_holiday_exception),
        )
        .layer(Extension(user))
        .layer(Extension(holiday_exception_service))
        .with_state(state)
}

#[tokio::test]
async fn test_admin_can_create_holiday_exception() {
    let _guard = integration_guard().await;
    let pool = setup_test_pool().await;

    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let token = create_test_token(admin.id, admin.role.clone());
    let app = test_router_with_state(pool.clone(), admin.clone());

    let payload = json!({
        "exception_date": "2024-12-25",
        "reason": "Working on holiday"
    });
    let response = oneshot_with_retry(&app, || {
        Request::builder()
            .method("POST")
            .uri(format!(
                "/api/admin/users/{}/holiday-exceptions",
                employee.id
            ))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap()
    })
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_system_admin_can_create_holiday_exception() {
    let _guard = integration_guard().await;
    let pool = setup_test_pool().await;

    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let app = test_router_with_state(pool.clone(), sysadmin.clone());

    let payload = json!({
        "exception_date": "2024-01-01",
        "reason": "Non-working day override"
    });
    let response = oneshot_with_retry(&app, || {
        Request::builder()
            .method("POST")
            .uri(format!(
                "/api/admin/users/{}/holiday-exceptions",
                employee.id
            ))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap()
    })
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_employee_cannot_create_holiday_exception() {
    let _guard = integration_guard().await;
    let pool = setup_test_pool().await;

    let employee1 = seed_user(&pool, UserRole::Employee, false).await;
    let employee2 = seed_user(&pool, UserRole::Employee, false).await;

    let token = create_test_token(employee1.id, employee1.role.clone());
    let app = test_router_with_state(pool.clone(), employee1.clone());

    let payload = json!({
        "exception_date": "2024-12-25",
        "reason": "Working"
    });
    let response = oneshot_with_retry(&app, || {
        Request::builder()
            .method("POST")
            .uri(format!(
                "/api/admin/users/{}/holiday-exceptions",
                employee2.id
            ))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap()
    })
    .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_admin_can_list_holiday_exceptions() {
    let _guard = integration_guard().await;
    let pool = setup_test_pool().await;

    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let token = create_test_token(admin.id, admin.role.clone());
    let app = test_router_with_state(pool.clone(), admin.clone());

    let response = oneshot_with_retry(&app, || {
        Request::builder()
            .uri(format!(
                "/api/admin/users/{}/holiday-exceptions",
                employee.id
            ))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap()
    })
    .await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_list_holiday_exceptions_with_date_range() {
    let _guard = integration_guard().await;
    let pool = setup_test_pool().await;

    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let token = create_test_token(admin.id, admin.role.clone());
    let app = test_router_with_state(pool.clone(), admin.clone());

    let response = oneshot_with_retry(&app, || {
        Request::builder()
            .uri(format!(
                "/api/admin/users/{}/holiday-exceptions?from=2024-01-01&to=2024-12-31",
                employee.id
            ))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap()
    })
    .await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_create_duplicate_holiday_exception_fails() {
    let _guard = integration_guard().await;
    let pool = setup_test_pool().await;

    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let token = create_test_token(admin.id, admin.role.clone());
    let app = test_router_with_state(pool.clone(), admin.clone());

    let payload = json!({
        "exception_date": "2024-07-04",
        "reason": "Working on holiday"
    });

    let first_response = oneshot_with_retry(&app, || {
        Request::builder()
            .method("POST")
            .uri(format!(
                "/api/admin/users/{}/holiday-exceptions",
                employee.id
            ))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap()
    })
    .await;
    assert_eq!(first_response.status(), StatusCode::CREATED);

    let response = oneshot_with_retry(&app, || {
        Request::builder()
            .method("POST")
            .uri(format!(
                "/api/admin/users/{}/holiday-exceptions",
                employee.id
            ))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap()
    })
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_invalid_user_id_format_fails() {
    let _guard = integration_guard().await;
    let pool = setup_test_pool().await;

    let admin = seed_user(&pool, UserRole::Admin, false).await;

    let token = create_test_token(admin.id, admin.role.clone());
    let app = test_router_with_state(pool.clone(), admin.clone());

    let payload = json!({
        "exception_date": "2024-12-25",
        "reason": "Working"
    });
    let response = oneshot_with_retry(&app, || {
        Request::builder()
            .method("POST")
            .uri("/api/admin/users/invalid-id/holiday-exceptions")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap()
    })
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_nonexistent_user_fails() {
    let _guard = integration_guard().await;
    let pool = setup_test_pool().await;

    let admin = seed_user(&pool, UserRole::Admin, false).await;

    let token = create_test_token(admin.id, admin.role.clone());
    let app = test_router_with_state(pool.clone(), admin.clone());

    let payload = json!({
        "exception_date": "2024-12-25",
        "reason": "Working"
    });
    let response = oneshot_with_retry(&app, || {
        Request::builder()
            .method("POST")
            .uri(format!(
                "/api/admin/users/{}/holiday-exceptions",
                timekeeper_backend::types::UserId::new()
            ))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap()
    })
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_employee_cannot_list_other_user_exceptions() {
    let _guard = integration_guard().await;
    let pool = setup_test_pool().await;

    let employee1 = seed_user(&pool, UserRole::Employee, false).await;
    let employee2 = seed_user(&pool, UserRole::Employee, false).await;

    let token = create_test_token(employee1.id, employee1.role.clone());
    let app = test_router_with_state(pool.clone(), employee1.clone());

    let response = oneshot_with_retry(&app, || {
        Request::builder()
            .uri(format!(
                "/api/admin/users/{}/holiday-exceptions",
                employee2.id
            ))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap()
    })
    .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
