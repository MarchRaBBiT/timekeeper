use axum::{
    extract::{Query, State},
    Extension,
};
use chrono::NaiveDate;
use serde_json::Value;
use timekeeper_backend::{
    handlers::{
        admin::{list_requests, RequestListQuery},
        requests::get_my_requests,
    },
    models::{leave_request::LeaveType, user::UserRole},
};

mod support;
use support::{seed_leave_request, seed_overtime_request, seed_user, setup_test_pool, test_config};

fn init_tracing() {
    let _ = tracing_subscriber::fmt::try_init();
}

#[tokio::test]
async fn get_my_requests_returns_leave_and_overtime() {
    let Some(pool) = setup_test_pool().await else {
        eprintln!("Skipping get_my_requests_returns_leave_and_overtime: database unavailable");
        return;
    };
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

#[tokio::test]
async fn admin_list_requests_includes_seeded_records() {
    let Some(pool) = setup_test_pool().await else {
        eprintln!("Skipping admin_list_requests_includes_seeded_records: database unavailable");
        return;
    };
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
