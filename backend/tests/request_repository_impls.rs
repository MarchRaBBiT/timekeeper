use chrono::{NaiveDate, Utc};
use std::sync::OnceLock;
use timekeeper_backend::{
    error::AppError,
    models::{
        leave_request::{LeaveRequest, LeaveType},
        overtime_request::OvertimeRequest,
        request::RequestStatus,
        user::UserRole,
    },
    repositories::{
        leave_request_repository::{LeaveRequestRepository, LeaveRequestRepositoryTrait},
        overtime_request_repository::{OvertimeRequestRepository, OvertimeRequestRepositoryTrait},
    },
};
use tokio::sync::Mutex;

#[path = "support/mod.rs"]
mod support;

async fn integration_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD.get_or_init(|| Mutex::new(())).lock().await
}

async fn reset_tables(pool: &sqlx::PgPool) {
    sqlx::query("TRUNCATE leave_requests, overtime_requests, users RESTART IDENTITY CASCADE")
        .execute(pool)
        .await
        .expect("truncate request tables");
}

#[tokio::test]
async fn leave_request_repository_impl_covers_crud_and_state_changes() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_tables(&pool).await;

    let user = support::seed_user(&pool, UserRole::Employee, false).await;
    let admin = support::seed_user(&pool, UserRole::Admin, false).await;
    let repo = LeaveRequestRepository::new();

    let first_start = NaiveDate::from_ymd_opt(2026, 3, 1).expect("valid date");
    let first_end = NaiveDate::from_ymd_opt(2026, 3, 2).expect("valid date");
    let first = LeaveRequest::new(
        user.id,
        LeaveType::Annual,
        first_start,
        first_end,
        Some("annual leave".to_string()),
    );
    let first = repo
        .create(&pool, &first)
        .await
        .expect("create first leave");

    assert_eq!(repo.find_all(&pool).await.expect("find all").len(), 1);
    assert_eq!(
        repo.find_by_user(&pool, user.id)
            .await
            .expect("find by user")
            .len(),
        1
    );
    assert_eq!(
        repo.find_by_user_and_date_range(
            &pool,
            user.id,
            NaiveDate::from_ymd_opt(2026, 2, 1).expect("valid date"),
            NaiveDate::from_ymd_opt(2026, 12, 31).expect("valid date"),
        )
        .await
        .expect("find by date range")
        .len(),
        1
    );
    assert!(repo
        .find_by_id_for_user(&pool, first.id, user.id)
        .await
        .expect("find by id for owner")
        .is_some());
    assert!(repo
        .find_by_id_for_user(&pool, first.id, admin.id)
        .await
        .expect("find by id for non-owner")
        .is_none());

    let missing = timekeeper_backend::types::LeaveRequestId::new();
    let missing_error = repo
        .find_by_id(&pool, missing)
        .await
        .expect_err("missing request should fail");
    assert!(matches!(missing_error, AppError::NotFound(_)));

    let now = Utc::now();
    assert_eq!(
        repo.approve(&pool, first.id, admin.id, "approved", now)
            .await
            .expect("approve request"),
        1
    );
    assert_eq!(
        repo.reject(&pool, first.id, admin.id, "already approved", now)
            .await
            .expect("reject approved request should not update"),
        0
    );
    assert_eq!(
        repo.cancel(&pool, first.id, user.id, now)
            .await
            .expect("cancel approved request should not update"),
        0
    );

    let second = LeaveRequest::new(
        user.id,
        LeaveType::Other,
        NaiveDate::from_ymd_opt(2026, 4, 1).expect("valid date"),
        NaiveDate::from_ymd_opt(2026, 4, 1).expect("valid date"),
        Some("other leave".to_string()),
    );
    let second = repo
        .create(&pool, &second)
        .await
        .expect("create second leave");
    let mut second_update = second.clone();
    second_update.leave_type = LeaveType::Sick;
    second_update.reason = Some("updated sick leave".to_string());
    second_update.status = RequestStatus::Pending;
    second_update.updated_at = Utc::now();
    let updated = repo
        .update(&pool, &second_update)
        .await
        .expect("update second leave");
    assert!(matches!(updated.leave_type, LeaveType::Sick));
    assert_eq!(updated.reason.as_deref(), Some("updated sick leave"));

    assert_eq!(
        repo.cancel(&pool, second.id, user.id, Utc::now())
            .await
            .expect("cancel pending request"),
        1
    );

    let third = LeaveRequest::new(
        user.id,
        LeaveType::Personal,
        NaiveDate::from_ymd_opt(2026, 5, 1).expect("valid date"),
        NaiveDate::from_ymd_opt(2026, 5, 2).expect("valid date"),
        Some("personal leave".to_string()),
    );
    let third = repo
        .create(&pool, &third)
        .await
        .expect("create third leave");
    assert_eq!(
        repo.reject(&pool, third.id, admin.id, "rejected", Utc::now())
            .await
            .expect("reject pending request"),
        1
    );

    repo.delete(&pool, first.id)
        .await
        .expect("delete first leave request");
    repo.delete(&pool, second.id)
        .await
        .expect("delete second leave request");
    repo.delete(&pool, third.id)
        .await
        .expect("delete third leave request");
    assert!(
        matches!(
            repo.find_by_id(&pool, first.id).await,
            Err(AppError::NotFound(_))
        ),
        "deleted request should be not found"
    );
}

#[tokio::test]
async fn overtime_request_repository_impl_covers_crud_and_state_changes() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_tables(&pool).await;

    let user = support::seed_user(&pool, UserRole::Employee, false).await;
    let admin = support::seed_user(&pool, UserRole::Admin, false).await;
    let repo = OvertimeRequestRepository::new();

    let first = OvertimeRequest::new(
        user.id,
        NaiveDate::from_ymd_opt(2026, 6, 1).expect("valid date"),
        2.5,
        Some("release support".to_string()),
    );
    let first = repo
        .create(&pool, &first)
        .await
        .expect("create first overtime");

    assert_eq!(repo.find_all(&pool).await.expect("find all").len(), 1);
    assert_eq!(
        repo.find_by_user(&pool, user.id)
            .await
            .expect("find by user")
            .len(),
        1
    );
    assert_eq!(
        repo.find_by_user_and_date_range(
            &pool,
            user.id,
            NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date"),
            NaiveDate::from_ymd_opt(2026, 12, 31).expect("valid date"),
        )
        .await
        .expect("find by date range")
        .len(),
        1
    );
    assert!(repo
        .find_by_id_for_user(&pool, first.id, user.id)
        .await
        .expect("find by id for owner")
        .is_some());
    assert!(repo
        .find_by_id_for_user(&pool, first.id, admin.id)
        .await
        .expect("find by id for non-owner")
        .is_none());

    let missing = timekeeper_backend::types::OvertimeRequestId::new();
    let missing_error = repo
        .find_by_id(&pool, missing)
        .await
        .expect_err("missing request should fail");
    assert!(matches!(missing_error, AppError::NotFound(_)));

    let now = Utc::now();
    assert_eq!(
        repo.approve(&pool, first.id, admin.id, "approved", now)
            .await
            .expect("approve request"),
        1
    );
    assert_eq!(
        repo.reject(&pool, first.id, admin.id, "already approved", now)
            .await
            .expect("reject approved request should not update"),
        0
    );
    assert_eq!(
        repo.cancel(&pool, first.id, user.id, now)
            .await
            .expect("cancel approved request should not update"),
        0
    );

    let second = OvertimeRequest::new(
        user.id,
        NaiveDate::from_ymd_opt(2026, 7, 1).expect("valid date"),
        3.0,
        Some("updated later".to_string()),
    );
    let second = repo
        .create(&pool, &second)
        .await
        .expect("create second overtime");
    let mut second_update = second.clone();
    second_update.status = RequestStatus::Pending;
    second_update.reason = Some("pending update".to_string());
    second_update.updated_at = Utc::now();
    let updated = repo
        .update(&pool, &second_update)
        .await
        .expect("update second overtime");
    assert!(matches!(updated.status, RequestStatus::Pending));
    assert_eq!(updated.reason.as_deref(), Some("pending update"));

    assert_eq!(
        repo.cancel(&pool, second.id, user.id, Utc::now())
            .await
            .expect("cancel pending overtime"),
        1
    );

    let third = OvertimeRequest::new(
        user.id,
        NaiveDate::from_ymd_opt(2026, 8, 1).expect("valid date"),
        1.0,
        Some("reject target".to_string()),
    );
    let third = repo
        .create(&pool, &third)
        .await
        .expect("create third overtime");
    assert_eq!(
        repo.reject(&pool, third.id, admin.id, "rejected", Utc::now())
            .await
            .expect("reject pending overtime"),
        1
    );

    repo.delete(&pool, first.id)
        .await
        .expect("delete first overtime");
    repo.delete(&pool, second.id)
        .await
        .expect("delete second overtime");
    repo.delete(&pool, third.id)
        .await
        .expect("delete third overtime");
    assert!(
        matches!(
            repo.find_by_id(&pool, first.id).await,
            Err(AppError::NotFound(_))
        ),
        "deleted request should be not found"
    );
}
