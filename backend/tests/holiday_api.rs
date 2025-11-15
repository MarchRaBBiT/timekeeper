use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::NaiveDate;
use sqlx::PgPool;
use timekeeper_backend::{
    handlers::{admin, holidays},
    models::{holiday::CreateWeeklyHolidayPayload, user::UserRole},
    services::holiday::HolidayService,
};

mod support;
use support::{seed_user, seed_weekly_holiday, test_config};

#[sqlx::test(migrations = "./migrations")]
async fn regular_admin_cannot_backdate_weekly_holiday(pool: PgPool) {
    let admin_user = seed_user(&pool, UserRole::Admin, false).await;
    let config = test_config();
    let backdated = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

    let payload = CreateWeeklyHolidayPayload {
        weekday: 1,
        starts_on: backdated,
        ends_on: None,
    };

    let result = admin::create_weekly_holiday(
        State((pool.clone(), config.clone())),
        Extension(admin_user.clone()),
        Json(payload),
    )
    .await;

    let err = result.expect_err("expected validation error for backdated start");
    assert_eq!(err.0, StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "./migrations")]
async fn system_admin_can_backdate_weekly_holiday(pool: PgPool) {
    let admin_user = seed_user(&pool, UserRole::Admin, true).await;
    let config = test_config();
    let backdated = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

    let payload = CreateWeeklyHolidayPayload {
        weekday: 1,
        starts_on: backdated,
        ends_on: None,
    };

    let response = admin::create_weekly_holiday(
        State((pool.clone(), config.clone())),
        Extension(admin_user.clone()),
        Json(payload),
    )
    .await
    .expect("system admin should succeed");

    assert_eq!(response.0.weekday, 1);
    assert_eq!(response.0.starts_on, backdated);
}

#[sqlx::test(migrations = "./migrations")]
async fn holiday_check_endpoint_detects_weekly_rule(pool: PgPool) {
    let user = seed_user(&pool, UserRole::Employee, false).await;
    let target_date = NaiveDate::from_ymd_opt(2025, 1, 8).unwrap();
    seed_weekly_holiday(&pool, target_date).await;

    let holiday_service = Arc::new(HolidayService::new(pool.clone()));

    let response = holidays::check_holiday(
        Extension(user.clone()),
        Extension(holiday_service.clone()),
        Query(holidays::HolidayCheckQuery { date: target_date }),
    )
    .await
    .expect("check call should succeed");

    assert!(response.0.is_holiday);
    assert!(response
        .0
        .reason
        .as_deref()
        .unwrap_or("")
        .contains("holiday"));
}
