use chrono::{NaiveDate, Utc};
use std::sync::OnceLock;
use timekeeper_backend::{
    models::{
        leave_request::{LeaveRequest, LeaveType, RequestStatus},
        overtime_request::OvertimeRequest,
        user::UserRole,
    },
    repositories::{
        LeaveRequestRepository, LeaveRequestRepositoryTrait, OvertimeRequestRepository,
        OvertimeRequestRepositoryTrait,
    },
};
use tokio::sync::Mutex;

#[path = "support/mod.rs"]
mod support;

async fn integration_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD.get_or_init(|| Mutex::new(())).lock().await
}

#[tokio::test]
async fn leave_request_repository_approve_roundtrip() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    sqlx::query("TRUNCATE leave_requests")
        .execute(&pool)
        .await
        .expect("truncate leave_requests");

    let user = support::seed_user(&pool, UserRole::Employee, false).await;
    let admin = support::seed_user(&pool, UserRole::Admin, false).await;
    let start = NaiveDate::from_ymd_opt(2024, 5, 1).unwrap();
    let end = NaiveDate::from_ymd_opt(2024, 5, 2).unwrap();
    let request = LeaveRequest::new(user.id, LeaveType::Annual, start, end, Some("trip".into()));

    let repo = LeaveRequestRepository::new();
    let saved = repo
        .create(&pool, &request)
        .await
        .expect("create leave request");
    assert_eq!(saved.user_id, user.id);
    assert!(matches!(saved.status, RequestStatus::Pending));

    let fetched = repo
        .find_by_id_for_user(&pool, saved.id, user.id)
        .await
        .expect("fetch leave request");
    assert!(fetched.is_some());

    let now = Utc::now();
    let updated = repo
        .approve(&pool, saved.id, admin.id, "ok", now)
        .await
        .expect("approve leave request");
    assert_eq!(updated, 1);

    let approved = repo
        .find_by_id(&pool, saved.id)
        .await
        .expect("fetch approved request");
    assert!(matches!(approved.status, RequestStatus::Approved));
    assert_eq!(approved.approved_by, Some(admin.id));
}

#[tokio::test]
async fn overtime_request_repository_reject_roundtrip() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    sqlx::query("TRUNCATE overtime_requests")
        .execute(&pool)
        .await
        .expect("truncate overtime_requests");

    let user = support::seed_user(&pool, UserRole::Employee, false).await;
    let admin = support::seed_user(&pool, UserRole::Admin, false).await;
    let date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let request = OvertimeRequest::new(user.id, date, 2.5, Some("release".into()));

    let repo = OvertimeRequestRepository::new();
    let saved = repo
        .create(&pool, &request)
        .await
        .expect("create overtime request");
    assert_eq!(saved.user_id, user.id);
    assert!(matches!(saved.status, RequestStatus::Pending));

    let now = Utc::now();
    let updated = repo
        .reject(&pool, saved.id, admin.id, "busy", now)
        .await
        .expect("reject overtime request");
    assert_eq!(updated, 1);

    let rejected = repo
        .find_by_id(&pool, saved.id)
        .await
        .expect("fetch rejected request");
    assert!(matches!(rejected.status, RequestStatus::Rejected));
    assert_eq!(rejected.rejected_by, Some(admin.id));
}
