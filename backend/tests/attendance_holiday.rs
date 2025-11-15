use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Extension, Json};
use chrono::NaiveDate;
use sqlx::PgPool;
use timekeeper_backend::{
    handlers::attendance,
    models::{attendance::ClockInRequest, user::UserRole},
    services::holiday::HolidayService,
};
use uuid::Uuid;

mod support;
use support::{seed_user, seed_weekly_holiday, test_config};

#[sqlx::test(migrations = "./migrations")]
async fn clock_in_blocked_on_weekly_holiday(pool: PgPool) {
    let user = seed_user(&pool, UserRole::Employee, false).await;
    let date = NaiveDate::from_ymd_opt(2025, 1, 8).unwrap();
    seed_weekly_holiday(&pool, date).await;

    let config = test_config();
    let holiday_service = Arc::new(HolidayService::new(pool.clone()));

    let result = attendance::clock_in(
        State((pool.clone(), config.clone())),
        Extension(user.clone()),
        Extension(holiday_service.clone()),
        Json(ClockInRequest { date: Some(date) }),
    )
    .await;

    let err = result.expect_err("should block clock-in on holiday");
    assert_eq!(err.0, StatusCode::FORBIDDEN);
    assert!(
        err.1
             .0
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .contains("holiday"),
        "expected error message to mention holiday"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn clock_in_allowed_with_exception(pool: PgPool) {
    let user = seed_user(&pool, UserRole::Employee, false).await;
    let date = NaiveDate::from_ymd_opt(2025, 1, 8).unwrap();
    seed_weekly_holiday(&pool, date).await;

    sqlx::query(
        "INSERT INTO holiday_exceptions \
            (id, user_id, exception_date, override, reason, created_by, created_at, updated_at) \
         VALUES ($1, $2, $3, FALSE, 'Work on holiday', 'test', NOW(), NOW())",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&user.id)
    .bind(date)
    .execute(&pool)
    .await
    .expect("insert exception");

    let config = test_config();
    let holiday_service = Arc::new(HolidayService::new(pool.clone()));

    let result = attendance::clock_in(
        State((pool.clone(), config.clone())),
        Extension(user.clone()),
        Extension(holiday_service.clone()),
        Json(ClockInRequest { date: Some(date) }),
    )
    .await;

    assert!(
        result.is_ok(),
        "clock-in should succeed when exception cancels holiday"
    );
}
