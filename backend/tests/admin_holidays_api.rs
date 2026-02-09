use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::get,
    Extension, Router,
};
use chrono::NaiveDate;
use serde_json::json;
use sqlx::PgPool;
use timekeeper_backend::{
    handlers::admin::holidays,
    models::holiday::{CreateHolidayPayload, CreateWeeklyHolidayPayload},
    models::user::{User, UserRole},
    state::AppState,
};
use tower::ServiceExt;

mod support;

use support::{create_test_token, seed_public_holiday, seed_user, test_config, test_pool};

async fn integration_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static GUARD: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();
    GUARD
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

fn test_router_with_state(pool: PgPool, user: User) -> Router {
    let state = AppState::new(pool, None, None, None, test_config());
    Router::new()
        .route(
            "/api/admin/holidays",
            get(holidays::list_holidays).post(holidays::create_holiday),
        )
        .route(
            "/api/admin/holidays/{id}",
            axum::routing::delete(holidays::delete_holiday),
        )
        .route(
            "/api/admin/holidays/weekly",
            get(holidays::list_weekly_holidays).post(holidays::create_weekly_holiday),
        )
        .route(
            "/api/admin/holidays/weekly/{id}",
            axum::routing::delete(holidays::delete_weekly_holiday),
        )
        .layer(Extension(user))
        .with_state(state)
}

async fn list_holidays(pool: &PgPool, user: &User) -> StatusCode {
    let token = create_test_token(user.id, user.role.clone());
    let app = test_router_with_state(pool.clone(), user.clone());

    let request = Request::builder()
        .uri("/api/admin/holidays")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    response.status()
}

#[tokio::test]
async fn test_admin_can_list_holidays() {
    let pool = test_pool().await;
    let _guard = integration_guard().await;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let _holiday = seed_public_holiday(
        &pool,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        "New Year",
    )
    .await;

    let status = list_holidays(&pool, &admin).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_non_admin_cannot_list_holidays() {
    let pool = test_pool().await;
    let _guard = integration_guard().await;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let status = list_holidays(&pool, &employee).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_admin_can_create_public_holiday() {
    let pool = test_pool().await;
    let _guard = integration_guard().await;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let admin = seed_user(&pool, UserRole::Admin, false).await;

    let payload = CreateHolidayPayload {
        holiday_date: NaiveDate::from_ymd_opt(2024, 12, 25).unwrap(),
        name: "Christmas".to_string(),
        description: Some("Public holiday".to_string()),
    };

    let token = create_test_token(admin.id, admin.role.clone());
    let state = AppState::new(pool.clone(), None, None, None, test_config());
    let app = Router::new()
        .route(
            "/api/admin/holidays",
            axum::routing::post(holidays::create_holiday),
        )
        .layer(Extension(admin))
        .with_state(state);

    let request = Request::builder()
        .method("POST")
        .uri("/api/admin/holidays")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!(payload).to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_cannot_create_holiday_with_empty_name() {
    let pool = test_pool().await;
    let _guard = integration_guard().await;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let admin = seed_user(&pool, UserRole::Admin, false).await;

    let payload = CreateHolidayPayload {
        holiday_date: NaiveDate::from_ymd_opt(2024, 12, 25).unwrap(),
        name: "   ".to_string(),
        description: None,
    };

    let token = create_test_token(admin.id, admin.role.clone());
    let state = AppState::new(pool.clone(), None, None, None, test_config());
    let app = Router::new()
        .route(
            "/api/admin/holidays",
            axum::routing::post(holidays::create_holiday),
        )
        .layer(Extension(admin))
        .with_state(state);

    let request = Request::builder()
        .method("POST")
        .uri("/api/admin/holidays")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!(payload).to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_cannot_create_duplicate_holiday_date() {
    let pool = test_pool().await;
    let _guard = integration_guard().await;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let existing = seed_public_holiday(
        &pool,
        NaiveDate::from_ymd_opt(2024, 7, 4).unwrap(),
        "Independence Day",
    )
    .await;

    let payload = CreateHolidayPayload {
        holiday_date: existing.holiday_date,
        name: "Another Holiday".to_string(),
        description: None,
    };

    let token = create_test_token(admin.id, admin.role.clone());
    let state = AppState::new(pool.clone(), None, None, None, test_config());
    let app = Router::new()
        .route(
            "/api/admin/holidays",
            axum::routing::post(holidays::create_holiday),
        )
        .layer(Extension(admin))
        .with_state(state);

    let request = Request::builder()
        .method("POST")
        .uri("/api/admin/holidays")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!(payload).to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_admin_can_delete_holiday() {
    let pool = test_pool().await;
    let _guard = integration_guard().await;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let holiday = seed_public_holiday(
        &pool,
        NaiveDate::from_ymd_opt(2024, 12, 26).unwrap(),
        "Boxing Day",
    )
    .await;

    let token = create_test_token(admin.id, admin.role.clone());
    let state = AppState::new(pool.clone(), None, None, None, test_config());
    let app = Router::new()
        .route(
            "/api/admin/holidays/{id}",
            axum::routing::delete(holidays::delete_holiday),
        )
        .layer(Extension(admin))
        .with_state(state);

    let request = Request::builder()
        .method("DELETE")
        .uri(format!("/api/admin/holidays/{}", holiday.id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_cannot_create_weekly_holiday_with_invalid_weekday() {
    let pool = test_pool().await;
    let _guard = integration_guard().await;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let admin = seed_user(&pool, UserRole::Admin, false).await;

    let payload = CreateWeeklyHolidayPayload {
        weekday: 7,
        starts_on: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        ends_on: None,
    };

    let token = create_test_token(admin.id, admin.role.clone());
    let state = AppState::new(pool.clone(), None, None, None, test_config());
    let app = Router::new()
        .route(
            "/api/admin/holidays/weekly",
            axum::routing::post(holidays::create_weekly_holiday),
        )
        .layer(Extension(admin))
        .with_state(state);

    let request = Request::builder()
        .method("POST")
        .uri("/api/admin/holidays/weekly")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!(payload).to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_cannot_create_weekly_holiday_with_invalid_date_range() {
    let pool = test_pool().await;
    let _guard = integration_guard().await;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let admin = seed_user(&pool, UserRole::Admin, false).await;

    let payload = CreateWeeklyHolidayPayload {
        weekday: 1,
        starts_on: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
        ends_on: Some(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
    };

    let token = create_test_token(admin.id, admin.role.clone());
    let state = AppState::new(pool.clone(), None, None, None, test_config());
    let app = Router::new()
        .route(
            "/api/admin/holidays/weekly",
            axum::routing::post(holidays::create_weekly_holiday),
        )
        .layer(Extension(admin))
        .with_state(state);

    let request = Request::builder()
        .method("POST")
        .uri("/api/admin/holidays/weekly")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!(payload).to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
