use chrono::{DateTime, NaiveDate, Utc};
use timekeeper_app::{
    attendance::{
        AttendanceRepository, ClockIn, ClockInCommand, ClockInWorkdayResolver, ClockOut,
        ClockOutCommand, NewClockIn,
    },
    work_schedules::ResolveWorkday,
};
use timekeeper_backend::models::user::UserRole;
use timekeeper_infra_postgres::{
    attendance::AttendanceWorkflowRepository, work_schedules::WorkdayResolverPostgresRepository,
};
use uuid::Uuid;

mod support;

use support::{seed_user, seed_work_schedule_for_user, test_pool};

async fn integration_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static GUARD: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();
    GUARD
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

fn recorded_at() -> DateTime<Utc> {
    DateTime::from_timestamp(1_783_396_800, 0).expect("recorded at")
}

#[tokio::test]
async fn overnight_punches_share_previous_locked_workday_snapshot() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    seed_work_schedule_for_user(&pool, employee.id, "follow_weekly_pattern").await;
    let resolver_repository = WorkdayResolverPostgresRepository::new(pool.clone());
    let resolver = ResolveWorkday::new(
        resolver_repository.clone(),
        resolver_repository.clone(),
        resolver_repository,
    );
    let attendance_repository = AttendanceWorkflowRepository::new(pool.clone());
    let clock_in = ClockIn::new(attendance_repository.clone(), resolver);
    let punch_time = NaiveDate::from_ymd_opt(2026, 7, 7)
        .expect("punch date")
        .and_hms_opt(2, 0, 0)
        .expect("clock in time");

    let clocked_in = clock_in
        .execute(ClockInCommand {
            user_id: employee.id.to_string(),
            requested_work_date: None,
            clock_in_time: punch_time,
            recorded_at: recorded_at(),
        })
        .await
        .expect("clock in");

    assert_eq!(clocked_in.work_date.to_string(), "2026-07-06");
    let linked: (Uuid, bool, Option<DateTime<Utc>>) = sqlx::query_as(
        "SELECT a.resolved_workday_id, a.is_unscheduled_work, r.locked_at \
         FROM attendance a JOIN resolved_workdays r ON r.id = a.resolved_workday_id \
         WHERE a.id = $1",
    )
    .bind(&clocked_in.attendance_id)
    .fetch_one(&pool)
    .await
    .expect("linked workday");
    assert!(!linked.1);
    assert_eq!(linked.2, Some(recorded_at()));

    let clocked_out = ClockOut::new(attendance_repository)
        .execute(ClockOutCommand {
            user_id: employee.id.to_string(),
            requested_work_date: None,
            clock_out_time: NaiveDate::from_ymd_opt(2026, 7, 7)
                .expect("clock-out date")
                .and_hms_opt(7, 0, 0)
                .expect("clock-out time"),
            recorded_at: recorded_at(),
        })
        .await
        .expect("clock out");
    assert_eq!(clocked_out.attendance_id, clocked_in.attendance_id);
    assert_eq!(clocked_out.work_date.to_string(), "2026-07-06");
}

#[tokio::test]
async fn database_rejects_attendance_linked_to_another_users_workday() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let owner = seed_user(&pool, UserRole::Employee, false).await;
    let other = seed_user(&pool, UserRole::Employee, false).await;
    seed_work_schedule_for_user(&pool, owner.id, "follow_weekly_pattern").await;
    let resolver_repository = WorkdayResolverPostgresRepository::new(pool.clone());
    let resolver = ResolveWorkday::new(
        resolver_repository.clone(),
        resolver_repository.clone(),
        resolver_repository,
    );
    let workday = ClockInWorkdayResolver::resolve_for_punch(
        &resolver,
        &owner.id.to_string(),
        Some(timekeeper_domain::WorkDate::from_ymd(2026, 7, 6).expect("work date")),
        NaiveDate::from_ymd_opt(2026, 7, 6)
            .expect("date")
            .and_hms_opt(9, 0, 0)
            .expect("punch time"),
        recorded_at(),
    )
    .await
    .expect("resolve workday");
    sqlx::query("UPDATE resolved_workdays SET locked_at = $2 WHERE id = $1")
        .bind(Uuid::parse_str(&workday.resolved_workday_id).expect("resolved id"))
        .bind(recorded_at())
        .execute(&pool)
        .await
        .expect("lock workday");

    let result = sqlx::query(
        "INSERT INTO attendance \
         (id, user_id, date, resolved_workday_id, clock_in_time, status) \
         VALUES ($1, $2, $3, $4, $5, 'present')",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(other.id.to_string())
    .bind(NaiveDate::from_ymd_opt(2026, 7, 6).expect("work date"))
    .bind(Uuid::parse_str(&workday.resolved_workday_id).expect("resolved id"))
    .bind(
        NaiveDate::from_ymd_opt(2026, 7, 6)
            .expect("date")
            .and_hms_opt(9, 0, 0)
            .expect("punch time"),
    )
    .execute(&pool)
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn failed_attendance_insert_rolls_back_workday_lock() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    seed_work_schedule_for_user(&pool, employee.id, "follow_weekly_pattern").await;
    let resolver_repository = WorkdayResolverPostgresRepository::new(pool.clone());
    let resolver = ResolveWorkday::new(
        resolver_repository.clone(),
        resolver_repository.clone(),
        resolver_repository,
    );
    let work_date = timekeeper_domain::WorkDate::from_ymd(2026, 7, 6).expect("work date");
    let punch_time = NaiveDate::from_ymd_opt(2026, 7, 6)
        .expect("date")
        .and_hms_opt(9, 0, 0)
        .expect("punch time");
    let workday = resolver
        .resolve_for_punch(
            &employee.id.to_string(),
            Some(work_date),
            punch_time,
            recorded_at(),
        )
        .await
        .expect("resolve workday");
    sqlx::query(
        "INSERT INTO attendance (id, user_id, date, status) VALUES ($1, $2, $3, 'present')",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(employee.id.to_string())
    .bind(work_date.as_naive_date())
    .execute(&pool)
    .await
    .expect("insert conflicting attendance");

    let result = AttendanceWorkflowRepository::new(pool.clone())
        .create_clock_in(NewClockIn {
            user_id: employee.id.to_string(),
            work_date,
            resolved_workday_id: workday.resolved_workday_id.clone(),
            clock_in_time: punch_time,
            recorded_at: recorded_at(),
        })
        .await;

    assert!(result.is_err());
    let locked_at: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT locked_at FROM resolved_workdays WHERE id = $1")
            .bind(Uuid::parse_str(&workday.resolved_workday_id).expect("resolved id"))
            .fetch_one(&pool)
            .await
            .expect("locked at");
    assert!(locked_at.is_none());
}

#[tokio::test]
async fn clock_in_reuses_an_already_locked_workday_snapshot() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    seed_work_schedule_for_user(&pool, employee.id, "follow_weekly_pattern").await;
    let resolver_repository = WorkdayResolverPostgresRepository::new(pool.clone());
    let resolver = ResolveWorkday::new(
        resolver_repository.clone(),
        resolver_repository.clone(),
        resolver_repository,
    );
    let work_date = timekeeper_domain::WorkDate::from_ymd(2026, 7, 6).expect("work date");
    let punch_time = NaiveDate::from_ymd_opt(2026, 7, 6)
        .expect("date")
        .and_hms_opt(9, 0, 0)
        .expect("punch time");
    let workday = resolver
        .resolve_for_punch(
            &employee.id.to_string(),
            Some(work_date),
            punch_time,
            recorded_at(),
        )
        .await
        .expect("resolve workday");
    sqlx::query("UPDATE resolved_workdays SET locked_at = $2 WHERE id = $1")
        .bind(Uuid::parse_str(&workday.resolved_workday_id).expect("resolved id"))
        .bind(recorded_at())
        .execute(&pool)
        .await
        .expect("pre-lock workday");

    let attendance = AttendanceWorkflowRepository::new(pool.clone())
        .create_clock_in(NewClockIn {
            user_id: employee.id.to_string(),
            work_date,
            resolved_workday_id: workday.resolved_workday_id.clone(),
            clock_in_time: punch_time,
            recorded_at: recorded_at(),
        })
        .await
        .expect("clock in with locked workday");

    let linked_workday_id: Uuid =
        sqlx::query_scalar("SELECT resolved_workday_id FROM attendance WHERE id = $1")
            .bind(&attendance.attendance_id)
            .fetch_one(&pool)
            .await
            .expect("linked workday");
    assert_eq!(linked_workday_id.to_string(), workday.resolved_workday_id);
}

#[tokio::test]
async fn clock_in_attaches_workday_to_existing_empty_attendance() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    seed_work_schedule_for_user(&pool, employee.id, "follow_weekly_pattern").await;
    let work_date = timekeeper_domain::WorkDate::from_ymd(2026, 7, 6).expect("work date");
    let attendance_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO attendance (id, user_id, date, status) VALUES ($1, $2, $3, 'present')",
    )
    .bind(&attendance_id)
    .bind(employee.id.to_string())
    .bind(work_date.as_naive_date())
    .execute(&pool)
    .await
    .expect("insert empty attendance");
    let resolver_repository = WorkdayResolverPostgresRepository::new(pool.clone());
    let resolver = ResolveWorkday::new(
        resolver_repository.clone(),
        resolver_repository.clone(),
        resolver_repository,
    );
    let punch_time = NaiveDate::from_ymd_opt(2026, 7, 6)
        .expect("date")
        .and_hms_opt(9, 0, 0)
        .expect("punch time");

    let result = ClockIn::new(AttendanceWorkflowRepository::new(pool.clone()), resolver)
        .execute(ClockInCommand {
            user_id: employee.id.to_string(),
            requested_work_date: Some(work_date),
            clock_in_time: punch_time,
            recorded_at: recorded_at(),
        })
        .await
        .expect("attach workday");

    assert_eq!(result.attendance_id, attendance_id);
    let linked: (Option<Uuid>, Option<DateTime<Utc>>) = sqlx::query_as(
        "SELECT a.resolved_workday_id, r.locked_at FROM attendance a \
         LEFT JOIN resolved_workdays r ON r.id = a.resolved_workday_id WHERE a.id = $1",
    )
    .bind(&attendance_id)
    .fetch_one(&pool)
    .await
    .expect("linked attendance");
    assert!(linked.0.is_some());
    assert_eq!(linked.1, Some(recorded_at()));
}
