use chrono::{NaiveDate, Utc};
use std::sync::OnceLock;
use timekeeper_backend::{
    models::{
        attendance::{Attendance, AttendanceStatus},
        break_record::BreakRecord,
        user::UserRole,
    },
    repositories::{
        repository::Repository,
        AttendanceRepository,
        AttendanceRepositoryTrait,
        BreakRecordRepository,
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
async fn attendance_repository_roundtrip() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    sqlx::query("TRUNCATE break_records, attendance")
        .execute(&pool)
        .await
        .expect("truncate attendance tables");

    let user = support::seed_user(&pool, UserRole::Employee, false).await;
    let now = Utc::now();
    let date = NaiveDate::from_ymd_opt(2024, 4, 1).unwrap();
    let mut attendance = Attendance::new(user.id, date, now);

    let repo = AttendanceRepository::new();
    let saved = repo
        .create(&pool, &attendance)
        .await
        .expect("create attendance");
    assert_eq!(saved.user_id, user.id);
    assert!(matches!(saved.status, AttendanceStatus::Present));

    attendance.clock_in_time = Some(date.and_hms_opt(9, 0, 0).unwrap());
    attendance.status = AttendanceStatus::Late;
    attendance.updated_at = now;
    let updated = repo
        .update(&pool, &attendance)
        .await
        .expect("update attendance");
    assert!(matches!(updated.status, AttendanceStatus::Late));
    assert!(updated.clock_in_time.is_some());

    let fetched = repo
        .find_by_user_and_date(&pool, user.id, date)
        .await
        .expect("find by user date");
    assert!(fetched.is_some());
}

#[tokio::test]
async fn break_record_repository_roundtrip() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    sqlx::query("TRUNCATE break_records, attendance")
        .execute(&pool)
        .await
        .expect("truncate attendance tables");

    let user = support::seed_user(&pool, UserRole::Employee, false).await;
    let now = Utc::now();
    let date = NaiveDate::from_ymd_opt(2024, 3, 15).unwrap();
    let attendance = Attendance::new(user.id, date, now);
    let attendance_repo = AttendanceRepository::new();
    let saved_attendance = attendance_repo
        .create(&pool, &attendance)
        .await
        .expect("create attendance");

    let start = date.and_hms_opt(12, 0, 0).unwrap();
    let mut record = BreakRecord::new(saved_attendance.id, start, now);
    let repo = BreakRecordRepository::new();
    let saved = repo
        .create(&pool, &record)
        .await
        .expect("create break record");
    assert_eq!(saved.attendance_id, saved_attendance.id);
    assert!(saved.duration_minutes.is_none());

    let end = start + chrono::Duration::minutes(15);
    record.break_end_time = Some(end);
    record.duration_minutes = Some(15);
    record.updated_at = now;
    let updated = repo
        .update(&pool, &record)
        .await
        .expect("update break record");
    assert_eq!(updated.duration_minutes, Some(15));

    let listed = repo
        .find_by_attendance(&pool, saved_attendance.id)
        .await
        .expect("list break records");
    assert_eq!(listed.len(), 1);
}
