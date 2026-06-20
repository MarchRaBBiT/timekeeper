use chrono::{DateTime, NaiveDate, NaiveTime, Timelike, Utc};
use sqlx::PgPool;
use timekeeper_app::work_schedules::{
    ResolveWorkday, ResolveWorkdayCommand, ResolvedDayKind, WorkScheduleSource,
};
use timekeeper_backend::models::user::UserRole;
use timekeeper_infra_postgres::work_schedules::WorkdayResolverPostgresRepository;
use uuid::Uuid;

mod support;

use support::{seed_user, test_pool};

async fn integration_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static GUARD: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();
    GUARD
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

#[derive(Clone, Copy)]
struct SeededSchedule {
    id: Uuid,
    version_id: Uuid,
}

fn work_date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 7, 6).expect("valid Monday")
}

fn resolved_at() -> DateTime<Utc> {
    DateTime::from_timestamp(1_783_296_000, 0).expect("valid timestamp")
}

fn resolver(
    pool: &PgPool,
) -> ResolveWorkday<
    WorkdayResolverPostgresRepository,
    WorkdayResolverPostgresRepository,
    WorkdayResolverPostgresRepository,
> {
    let repository = WorkdayResolverPostgresRepository::new(pool.clone());
    ResolveWorkday::new(repository.clone(), repository.clone(), repository)
}

async fn reset_work_schedule_data(pool: &PgPool) {
    sqlx::query(
        "TRUNCATE TABLE workday_overrides, resolved_workdays, \
         work_schedule_assignments, work_schedules CASCADE",
    )
    .execute(pool)
    .await
    .expect("reset work schedule data");
}

async fn seed_schedule(
    pool: &PgPool,
    created_by: &str,
    policy: &str,
    start_time: NaiveTime,
    end_time: NaiveTime,
    end_day_offset: i16,
) -> SeededSchedule {
    let schedule_id = Uuid::new_v4();
    let version_id = Uuid::new_v4();
    sqlx::query("INSERT INTO work_schedules (id, code, name, created_by) VALUES ($1, $2, $3, $4)")
        .bind(schedule_id)
        .bind(format!("schedule-{schedule_id}"))
        .bind("Resolver test schedule")
        .bind(created_by)
        .execute(pool)
        .await
        .expect("insert schedule");
    sqlx::query(
        "INSERT INTO work_schedule_versions \
         (id, work_schedule_id, version_number, status, effective_from, timezone, \
          workday_boundary, public_holiday_policy, published_by, published_at) \
         VALUES ($1, $2, 1, 'published', $3, 'Asia/Tokyo', $4, $5, $6, $7)",
    )
    .bind(version_id)
    .bind(schedule_id)
    .bind(NaiveDate::from_ymd_opt(2026, 7, 1).expect("effective date"))
    .bind(NaiveTime::from_hms_opt(5, 0, 0).expect("boundary"))
    .bind(policy)
    .bind(created_by)
    .bind(resolved_at())
    .execute(pool)
    .await
    .expect("insert version");

    for weekday in 1_i16..=7 {
        let day_rule_id = Uuid::new_v4();
        let working = weekday == 1;
        sqlx::query(
            "INSERT INTO work_schedule_day_rules \
             (id, version_id, weekday, day_kind, expected_work_minutes) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(day_rule_id)
        .bind(version_id)
        .bind(weekday)
        .bind(if working {
            "working_day"
        } else {
            "non_working_day"
        })
        .bind(if working { 480 } else { 0 })
        .execute(pool)
        .await
        .expect("insert day rule");
        if working {
            sqlx::query(
                "INSERT INTO work_schedule_work_intervals \
                 (day_rule_id, sequence, start_time, start_day_offset, end_time, end_day_offset) \
                 VALUES ($1, 1, $2, 0, $3, $4)",
            )
            .bind(day_rule_id)
            .bind(start_time)
            .bind(end_time)
            .bind(end_day_offset)
            .execute(pool)
            .await
            .expect("insert interval");
            let (break_start, break_end, break_day_offset) = if end_day_offset == 1 {
                (
                    NaiveTime::from_hms_opt(2, 0, 0).expect("break start"),
                    NaiveTime::from_hms_opt(3, 0, 0).expect("break end"),
                    1_i16,
                )
            } else {
                (
                    NaiveTime::from_hms_opt(12, 0, 0).expect("break start"),
                    NaiveTime::from_hms_opt(13, 0, 0).expect("break end"),
                    0_i16,
                )
            };
            sqlx::query(
                "INSERT INTO work_schedule_planned_breaks \
                 (day_rule_id, sequence, start_time, start_day_offset, end_time, end_day_offset) \
                 VALUES ($1, 1, $2, $3, $4, $3)",
            )
            .bind(day_rule_id)
            .bind(break_start)
            .bind(break_day_offset)
            .bind(break_end)
            .execute(pool)
            .await
            .expect("insert break");
        }
    }
    SeededSchedule {
        id: schedule_id,
        version_id,
    }
}

async fn seed_assignment(
    pool: &PgPool,
    schedule_id: Uuid,
    created_by: &str,
    user_id: Option<&str>,
    department_id: Option<&str>,
    is_org_default: bool,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO work_schedule_assignments \
         (id, work_schedule_id, user_id, department_id, is_org_default, valid_from, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(id)
    .bind(schedule_id)
    .bind(user_id)
    .bind(department_id)
    .bind(is_org_default)
    .bind(NaiveDate::from_ymd_opt(2026, 7, 1).expect("valid from"))
    .bind(created_by)
    .execute(pool)
    .await
    .expect("insert assignment");
    id
}

async fn seed_department(pool: &PgPool, id: &str, parent_id: Option<&str>) {
    sqlx::query("INSERT INTO departments (id, name, parent_id) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(format!("Department {id}"))
        .bind(parent_id)
        .execute(pool)
        .await
        .expect("insert department");
}

fn command(user_id: String) -> ResolveWorkdayCommand {
    ResolveWorkdayCommand {
        user_id,
        work_date: work_date(),
        resolved_at: resolved_at(),
    }
}

#[tokio::test]
async fn resolves_nearest_department_then_replaces_unlocked_projection_with_user_assignment() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_work_schedule_data(&pool).await;
    let actor = seed_user(&pool, UserRole::Manager, true).await;
    let user = seed_user(&pool, UserRole::Employee, false).await;
    let parent_id = Uuid::new_v4().to_string();
    let child_id = Uuid::new_v4().to_string();
    seed_department(&pool, &parent_id, None).await;
    seed_department(&pool, &child_id, Some(&parent_id)).await;
    sqlx::query("UPDATE users SET department_id = $2 WHERE id = $1")
        .bind(user.id.to_string())
        .bind(&child_id)
        .execute(&pool)
        .await
        .expect("assign department");
    let org_schedule = seed_schedule(
        &pool,
        &actor.id.to_string(),
        "follow_weekly_pattern",
        NaiveTime::from_hms_opt(9, 0, 0).expect("start"),
        NaiveTime::from_hms_opt(18, 0, 0).expect("end"),
        0,
    )
    .await;
    let parent_schedule = seed_schedule(
        &pool,
        &actor.id.to_string(),
        "follow_weekly_pattern",
        NaiveTime::from_hms_opt(22, 0, 0).expect("start"),
        NaiveTime::from_hms_opt(7, 0, 0).expect("end"),
        1,
    )
    .await;
    let user_schedule = seed_schedule(
        &pool,
        &actor.id.to_string(),
        "follow_weekly_pattern",
        NaiveTime::from_hms_opt(8, 0, 0).expect("start"),
        NaiveTime::from_hms_opt(17, 0, 0).expect("end"),
        0,
    )
    .await;
    seed_assignment(
        &pool,
        org_schedule.id,
        &actor.id.to_string(),
        None,
        None,
        true,
    )
    .await;
    let parent_assignment = seed_assignment(
        &pool,
        parent_schedule.id,
        &actor.id.to_string(),
        None,
        Some(&parent_id),
        false,
    )
    .await;

    let first = resolver(&pool)
        .execute(command(user.id.to_string()))
        .await
        .expect("department resolution");
    assert_eq!(first.source, WorkScheduleSource::Department);
    assert_eq!(first.source_id, parent_assignment.to_string());
    assert_eq!(first.work_intervals[0].end_day_offset, 1);

    let user_assignment = seed_assignment(
        &pool,
        user_schedule.id,
        &actor.id.to_string(),
        Some(&user.id.to_string()),
        None,
        false,
    )
    .await;
    let second = resolver(&pool)
        .execute(command(user.id.to_string()))
        .await
        .expect("user resolution");
    assert_eq!(second.id, first.id);
    assert_eq!(second.source, WorkScheduleSource::User);
    assert_eq!(second.source_id, user_assignment.to_string());
    assert_eq!(second.work_intervals[0].start_time.hour(), 8);
}

#[tokio::test]
async fn override_has_priority_over_public_holiday() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_work_schedule_data(&pool).await;
    let actor = seed_user(&pool, UserRole::Manager, true).await;
    let user = seed_user(&pool, UserRole::Employee, false).await;
    let schedule = seed_schedule(
        &pool,
        &actor.id.to_string(),
        "non_working",
        NaiveTime::from_hms_opt(9, 0, 0).expect("start"),
        NaiveTime::from_hms_opt(18, 0, 0).expect("end"),
        0,
    )
    .await;
    seed_assignment(&pool, schedule.id, &actor.id.to_string(), None, None, true).await;
    sqlx::query(
        "INSERT INTO holidays (id, holiday_date, name) VALUES ($1, $2, 'Resolver holiday')",
    )
    .bind(Uuid::new_v4())
    .bind(work_date())
    .execute(&pool)
    .await
    .expect("insert holiday");

    let holiday = resolver(&pool)
        .execute(command(user.id.to_string()))
        .await
        .expect("holiday resolution");
    assert_eq!(holiday.day_kind, ResolvedDayKind::PublicHoliday);
    assert!(holiday.work_intervals.is_empty());

    let override_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workday_overrides \
         (id, user_id, work_date, kind, work_schedule_id, reason, created_by) \
         VALUES ($1, $2, $3, 'use_schedule', $4, 'scheduled holiday work', $5)",
    )
    .bind(override_id)
    .bind(user.id.to_string())
    .bind(work_date())
    .bind(schedule.id)
    .bind(actor.id.to_string())
    .execute(&pool)
    .await
    .expect("insert override");
    let overridden = resolver(&pool)
        .execute(command(user.id.to_string()))
        .await
        .expect("override resolution");
    assert_eq!(overridden.source, WorkScheduleSource::Override);
    assert_eq!(overridden.source_id, override_id.to_string());
    assert_eq!(overridden.day_kind, ResolvedDayKind::ScheduledWorkday);
    assert_eq!(overridden.work_intervals.len(), 1);
}

#[tokio::test]
async fn locked_projection_is_immutable_and_repeated_resolution_is_idempotent() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_work_schedule_data(&pool).await;
    let actor = seed_user(&pool, UserRole::Manager, true).await;
    let user = seed_user(&pool, UserRole::Employee, false).await;
    let schedule = seed_schedule(
        &pool,
        &actor.id.to_string(),
        "follow_weekly_pattern",
        NaiveTime::from_hms_opt(22, 0, 0).expect("start"),
        NaiveTime::from_hms_opt(7, 0, 0).expect("end"),
        1,
    )
    .await;
    seed_assignment(&pool, schedule.id, &actor.id.to_string(), None, None, true).await;
    let first = resolver(&pool)
        .execute(command(user.id.to_string()))
        .await
        .expect("first resolution");
    let second = resolver(&pool)
        .execute(command(user.id.to_string()))
        .await
        .expect("second resolution");
    assert_eq!(second.id, first.id);
    let interval_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM resolved_workday_intervals WHERE resolved_workday_id = $1",
    )
    .bind(Uuid::parse_str(&first.id).expect("resolved id"))
    .fetch_one(&pool)
    .await
    .expect("interval count");
    assert_eq!(interval_count, 1);

    sqlx::query("UPDATE resolved_workdays SET locked_at = $2 WHERE id = $1")
        .bind(Uuid::parse_str(&first.id).expect("resolved id"))
        .bind(resolved_at())
        .execute(&pool)
        .await
        .expect("lock projection");
    let locked = resolver(&pool)
        .execute(command(user.id.to_string()))
        .await
        .expect("locked resolution");
    assert_eq!(locked.id, first.id);
    assert_eq!(
        locked.work_schedule_version_id,
        schedule.version_id.to_string()
    );
    assert_eq!(locked.locked_at, Some(resolved_at()));

    let mutation =
        sqlx::query("UPDATE resolved_workdays SET expected_work_minutes = 1 WHERE id = $1")
            .bind(Uuid::parse_str(&first.id).expect("resolved id"))
            .execute(&pool)
            .await;
    assert!(mutation.is_err());
    let child_mutation = sqlx::query(
        "UPDATE resolved_workday_intervals SET start_time = $2 WHERE resolved_workday_id = $1",
    )
    .bind(Uuid::parse_str(&first.id).expect("resolved id"))
    .bind(NaiveTime::from_hms_opt(23, 0, 0).expect("new start"))
    .execute(&pool)
    .await;
    assert!(child_mutation.is_err());
}
