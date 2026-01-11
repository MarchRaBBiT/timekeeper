#![cfg(feature = "test-utils")]

use chrono::NaiveDate;
use timekeeper_backend::{
    handlers::{holiday_exception_repo, holiday_exceptions},
    models::{
        holiday_exception::{
            CreateHolidayExceptionPayload, HolidayException, HolidayExceptionResponse,
        },
        user::UserRole,
    },
    services::{
        holiday::{HolidayReason, HolidayService},
        holiday_exception::{HolidayExceptionService, HolidayExceptionServiceTrait},
    },
};

use {
    axum::{
        body::Body,
        extract::Extension,
        http::{Request, StatusCode},
        routing::{delete, get, post},
        Router,
    },
    hyper::body::to_bytes,
    serde_json::json,
    std::sync::Arc,
    tower::ServiceExt,
};

#[path = "support/mod.rs"]
mod support;

#[tokio::test]
async fn repository_prevents_duplicate_exceptions_for_same_user_and_date() {
    let pool = support::test_pool().await;
    sqlx::migrate!("../migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let user = support::seed_user(&pool, UserRole::Employee, false).await;
    let date = NaiveDate::from_ymd_opt(2025, 5, 1).unwrap();

    let first = HolidayException::new(
        user.id.clone(),
        date,
        Some("出社対応".into()),
        user.id.clone(),
    );
    holiday_exception_repo::insert_holiday_exception(&pool, &first)
        .await
        .expect("insert first exception");

    let second = HolidayException::new(
        user.id.clone(),
        date,
        Some("重複を許可しない".into()),
        user.id.clone(),
    );

    let result = holiday_exception_repo::insert_holiday_exception(&pool, &second).await;

    assert!(
        matches!(result, Err(sqlx::Error::Database(db_err)) if db_err.constraint() == Some("holiday_exceptions_user_date_key")),
        "duplicate insert should surface unique constraint"
    );
}

#[tokio::test]
async fn service_marks_exception_as_workday_over_public_and_weekly() {
    let pool = support::test_pool().await;
    sqlx::migrate!("../migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let user = support::seed_user(&pool, UserRole::Employee, false).await;
    let date = NaiveDate::from_ymd_opt(2025, 5, 3).unwrap();
    support::seed_public_holiday(&pool, date, "みなし公休日").await;
    support::seed_weekly_holiday(&pool, date).await;

    let exception_service = HolidayExceptionService::new(pool.clone());
    let holiday_service = HolidayService::new(pool.clone());

    exception_service
        .create_workday_override(
            &user.id,
            CreateHolidayExceptionPayload {
                exception_date: date,
                reason: Some("現場稼働のため出社".into()),
            },
            &user.id,
        )
        .await
        .expect("create override");

    let decision = holiday_service
        .is_holiday(date, Some(&user.id))
        .await
        .expect("holiday decision");

    assert!(
        !decision.is_holiday,
        "exception should mark the day as working day"
    );
    assert!(matches!(decision.reason, HolidayReason::ExceptionOverride));
}

#[tokio::test]
async fn handler_creates_and_lists_personal_workday_override() {
    let pool = support::test_pool().await;
    sqlx::migrate!("../migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let config = support::test_config();
    let admin = support::seed_user(&pool, UserRole::Admin, false).await;
    let target = support::seed_user(&pool, UserRole::Employee, false).await;

    let exception_service: Arc<dyn HolidayExceptionServiceTrait> =
        Arc::new(HolidayExceptionService::new(pool.clone()));

    let app = Router::new()
        .route(
            "/api/admin/users/:user_id/holiday-exceptions",
            post(holiday_exceptions::create_holiday_exception)
                .get(holiday_exceptions::list_holiday_exceptions),
        )
        .route(
            "/api/admin/users/:user_id/holiday-exceptions/:id",
            delete(holiday_exceptions::delete_holiday_exception),
        )
        .with_state((pool.clone(), config.clone()))
        .layer(Extension(admin.clone()))
        .layer(Extension(exception_service.clone()));

    let payload = json!({
        "exception_date": date(2025, 6, 10),
        "reason": "祝日を出社扱いにする"
    });
    let response = app
        .clone()
        .oneshot(
            Request::post(format!("/api/admin/users/{}/holiday-exceptions", target.id))
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .expect("handler should respond");
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = to_bytes(response.into_body()).await.expect("read body");
    let created: HolidayExceptionResponse =
        serde_json::from_slice(&body).expect("parse response body");
    assert_eq!(created.exception_date, date(2025, 6, 10));
    assert!(created.is_workday);
    assert_eq!(created.reason.as_deref(), Some("祝日を出社扱いにする"));

    let duplicate = app
        .clone()
        .oneshot(
            Request::post(format!("/api/admin/users/{}/holiday-exceptions", target.id))
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .expect("handler should respond");
    assert_eq!(duplicate.status(), StatusCode::CONFLICT);

    let list_response = app
        .oneshot(
            Request::get(format!("/api/admin/users/{}/holiday-exceptions", target.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("list handler should respond");
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_body = to_bytes(list_response.into_body())
        .await
        .expect("read list body");
    let listed: Vec<HolidayExceptionResponse> =
        serde_json::from_slice(&list_body).expect("parse list response");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].exception_date, date(2025, 6, 10));
}

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("valid date")
}
