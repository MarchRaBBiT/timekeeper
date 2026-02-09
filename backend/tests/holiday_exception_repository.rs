use chrono::NaiveDate;
use std::sync::OnceLock;
use timekeeper_backend::{
    models::{holiday_exception::HolidayException, user::UserRole},
    repositories::holiday_exception as holiday_exception_repo,
};
use tokio::sync::Mutex;

#[path = "support/mod.rs"]
mod support;

async fn integration_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD.get_or_init(|| Mutex::new(())).lock().await
}

async fn reset_tables(pool: &sqlx::PgPool) {
    sqlx::query("TRUNCATE holiday_exceptions, users RESTART IDENTITY CASCADE")
        .execute(pool)
        .await
        .expect("truncate holiday exception tables");
}

#[tokio::test]
async fn holiday_exception_repository_lists_with_range_and_deletes_by_owner() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_tables(&pool).await;

    let user = support::seed_user(&pool, UserRole::Employee, false).await;
    let other = support::seed_user(&pool, UserRole::Employee, false).await;
    let creator = support::seed_user(&pool, UserRole::Admin, false).await;

    let old_date = NaiveDate::from_ymd_opt(2026, 1, 5).expect("valid date");
    let in_range_date = NaiveDate::from_ymd_opt(2026, 1, 20).expect("valid date");

    let first = HolidayException::new(
        user.id,
        old_date,
        Some("old exception".to_string()),
        creator.id,
    );
    holiday_exception_repo::insert_holiday_exception(&pool, &first)
        .await
        .expect("insert old exception");

    let second = HolidayException::new(
        user.id,
        in_range_date,
        Some("in range exception".to_string()),
        creator.id,
    );
    holiday_exception_repo::insert_holiday_exception(&pool, &second)
        .await
        .expect("insert in-range exception");

    let listed = holiday_exception_repo::list_holiday_exceptions_for_user(
        &pool,
        user.id,
        Some(NaiveDate::from_ymd_opt(2026, 1, 10).expect("valid date")),
        Some(NaiveDate::from_ymd_opt(2026, 1, 31).expect("valid date")),
    )
    .await
    .expect("list exceptions in range");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, second.id);

    let delete_wrong_user =
        holiday_exception_repo::delete_holiday_exception(&pool, second.id, other.id)
            .await
            .expect("delete with wrong owner");
    assert_eq!(delete_wrong_user, 0);

    let delete_owner = holiday_exception_repo::delete_holiday_exception(&pool, second.id, user.id)
        .await
        .expect("delete with correct owner");
    assert_eq!(delete_owner, 1);

    let remaining =
        holiday_exception_repo::list_holiday_exceptions_for_user(&pool, user.id, None, None)
            .await
            .expect("list remaining exceptions");
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, first.id);
}
