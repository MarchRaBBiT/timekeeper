use chrono::{NaiveDate, Utc};
use std::sync::OnceLock;
use timekeeper_backend::{
    error::AppError,
    models::{holiday::WeeklyHoliday, user::UserRole},
    repositories::{repository::Repository, WeeklyHolidayRepository},
    types::WeeklyHolidayId,
};
use tokio::sync::Mutex;

#[path = "support/mod.rs"]
mod support;

async fn integration_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD.get_or_init(|| Mutex::new(())).lock().await
}

async fn reset_tables(pool: &sqlx::PgPool) {
    sqlx::query("TRUNCATE weekly_holidays, users RESTART IDENTITY CASCADE")
        .execute(pool)
        .await
        .expect("truncate weekly holiday tables");
}

#[tokio::test]
async fn weekly_holiday_repository_crud_roundtrip() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_tables(&pool).await;

    let user = support::seed_user(&pool, UserRole::Admin, false).await;
    let starts_on = NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date");
    let repo = WeeklyHolidayRepository::new();
    let created = repo
        .create(
            &pool,
            &WeeklyHoliday::new(1, starts_on, Some(starts_on), user.id),
        )
        .await
        .expect("create weekly holiday");
    assert_eq!(created.weekday, 1);
    assert_eq!(created.starts_on, starts_on);

    let found = repo
        .find_by_id(&pool, created.id)
        .await
        .expect("find weekly holiday by id");
    assert_eq!(found.id, created.id);

    let listed = repo.find_all(&pool).await.expect("list weekly holidays");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, created.id);

    let mut updating = found;
    updating.weekday = 4;
    updating.ends_on = None;
    updating.enforced_to = None;
    updating.updated_at = Utc::now();
    let updated = repo
        .update(&pool, &updating)
        .await
        .expect("update weekly holiday");
    assert_eq!(updated.weekday, 4);
    assert!(updated.ends_on.is_none());

    repo.delete(&pool, updated.id)
        .await
        .expect("delete weekly holiday");
    let listed = repo.find_all(&pool).await.expect("list after delete");
    assert!(listed.is_empty());
}

#[tokio::test]
async fn weekly_holiday_repository_returns_not_found_for_missing_rows() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_tables(&pool).await;

    let repo = WeeklyHolidayRepository::new();
    let missing_id = WeeklyHolidayId::new();

    let err = repo
        .find_by_id(&pool, missing_id)
        .await
        .expect_err("missing row must be not found");
    assert!(matches!(err, AppError::NotFound(_)));

    let err = repo
        .delete(&pool, missing_id)
        .await
        .expect_err("delete missing row must be not found");
    assert!(matches!(err, AppError::NotFound(_)));
}
