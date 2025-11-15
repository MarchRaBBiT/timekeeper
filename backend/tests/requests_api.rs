use axum::{
    extract::{Query, State},
    Extension,
};
use chrono::NaiveDate;
use serde_json::Value;
use sqlx::PgPool;
use timekeeper_backend::{
    handlers::{
        admin::{list_requests, RequestListQuery},
        requests::get_my_requests,
    },
    models::{leave_request::LeaveType, user::UserRole},
};

mod support;
use support::{seed_leave_request, seed_overtime_request, seed_user, test_config};

fn init_tracing() {
    let _ = tracing_subscriber::fmt::try_init();
}

#[sqlx::test(migrations = "./migrations")]
async fn get_my_requests_returns_leave_and_overtime(pool: PgPool) {
    init_tracing();
    let config = test_config();
    let user = seed_user(&pool, UserRole::Employee, false).await;
    let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    seed_leave_request(&pool, &user.id, LeaveType::Annual, date, date).await;
    seed_overtime_request(&pool, &user.id, date, 1.5).await;

    let response = get_my_requests(State((pool.clone(), config)), Extension(user.clone())).await;

    let payload: Value = response.expect("get_my_requests ok").0;
    let leave = payload
        .get("leave_requests")
        .and_then(|v| v.as_array())
        .expect("leave list");
    let overtime = payload
        .get("overtime_requests")
        .and_then(|v| v.as_array())
        .expect("overtime list");
    assert_eq!(leave.len(), 1);
    assert_eq!(overtime.len(), 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn admin_list_requests_includes_seeded_records(pool: PgPool) {
    init_tracing();
    let config = test_config();
    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let date = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();
    seed_leave_request(&pool, &employee.id, LeaveType::Sick, date, date).await;
    seed_overtime_request(&pool, &employee.id, date, 2.0).await;

    let query = RequestListQuery {
        status: None,
        r#type: None,
        user_id: None,
        from: None,
        to: None,
        page: Some(1),
        per_page: Some(20),
    };

    let response = list_requests(
        State((pool.clone(), config)),
        Extension(admin.clone()),
        Query(query),
    )
    .await;

    let payload: Value = response.expect("list_requests ok").0;
    let leave = payload
        .get("leave_requests")
        .and_then(|v| v.as_array())
        .expect("leave array");
    let overtime = payload
        .get("overtime_requests")
        .and_then(|v| v.as_array())
        .expect("overtime array");
    assert!(!leave.is_empty());
    assert!(!overtime.is_empty());
}
