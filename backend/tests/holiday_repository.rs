use chrono::{NaiveDate, Utc};
use std::sync::OnceLock;
use timekeeper_backend::{
    models::{holiday::Holiday, user::UserRole},
    repositories::{repository::Repository, HolidayRepository},
};
use tokio::sync::Mutex;

#[path = "support/mod.rs"]
mod support;

async fn integration_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD.get_or_init(|| Mutex::new(())).lock().await
}

#[tokio::test]
async fn holiday_repository_roundtrip() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    sqlx::query("TRUNCATE holidays")
        .execute(&pool)
        .await
        .expect("truncate holidays");

    let _user = support::seed_user(&pool, UserRole::Admin, false).await;
    let date = NaiveDate::from_ymd_opt(2024, 12, 25).unwrap();
    let holiday = Holiday::new(date, "Holiday".into(), Some("seasonal".into()));

    let repo = HolidayRepository::new();
    let saved = repo.create(&pool, &holiday).await.expect("create holiday");
    assert_eq!(saved.holiday_date, date);

    let by_date = repo.find_by_date(&pool, date).await.expect("find by date");
    assert!(by_date.is_some());

    let mut updated = saved;
    updated.name = "Updated".into();
    updated.description = None;
    updated.updated_at = Utc::now();
    let saved_update = repo.update(&pool, &updated).await.expect("update holiday");
    assert_eq!(saved_update.name, "Updated");
    assert!(saved_update.description.is_none());

    repo.delete(&pool, saved_update.id)
        .await
        .expect("delete holiday");
    let missing = repo
        .find_by_date(&pool, date)
        .await
        .expect("find after delete");
    assert!(missing.is_none());
}
