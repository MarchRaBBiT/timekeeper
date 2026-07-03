use chrono::{DateTime, NaiveDate, NaiveTime, Timelike, Utc};
use sqlx::PgPool;
use timekeeper_app::work_schedules::{
    ResolveWorkday, ResolveWorkdayCommand, ResolveWorkdayError, ResolvedDayKind, ScheduleType,
    WorkScheduleSource, WorkdayResolutionRepository,
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

async fn create_schedule(pool: &PgPool, created_by: &str) -> Uuid {
    let schedule_id = Uuid::new_v4();
    sqlx::query("INSERT INTO work_schedules (id, code, name, created_by) VALUES ($1, $2, $3, $4)")
        .bind(schedule_id)
        .bind(format!("schedule-{schedule_id}"))
        .bind("Resolver flex test schedule")
        .bind(created_by)
        .execute(pool)
        .await
        .expect("insert schedule");
    schedule_id
}

#[allow(clippy::too_many_arguments)]
async fn seed_flex_schedule(
    pool: &PgPool,
    created_by: &str,
    schedule_id: Uuid,
    version_number: i32,
    effective_from: NaiveDate,
    effective_until: Option<NaiveDate>,
    policy: &str,
    core_time: Option<(i16, NaiveTime, i16, NaiveTime, i16)>,
) -> Uuid {
    let version_id = Uuid::new_v4();
    // Insert as draft first: the published-version flex-immutability trigger
    // (migration 048) rejects INSERT into flex child tables once the parent
    // version's status is already 'published'.
    sqlx::query(
        "INSERT INTO work_schedule_versions \
         (id, work_schedule_id, version_number, status, effective_from, effective_until, \
          timezone, workday_boundary, public_holiday_policy, schedule_type) \
         VALUES ($1, $2, $3, 'draft', $4, $5, 'Asia/Tokyo', $6, $7, 'flex')",
    )
    .bind(version_id)
    .bind(schedule_id)
    .bind(version_number)
    .bind(effective_from)
    .bind(effective_until)
    .bind(NaiveTime::from_hms_opt(5, 0, 0).expect("boundary"))
    .bind(policy)
    .execute(pool)
    .await
    .expect("insert draft flex version");

    for weekday in 1_i16..=7 {
        let day_rule_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO work_schedule_day_rules \
             (id, version_id, weekday, day_kind, expected_work_minutes) \
             VALUES ($1, $2, $3, 'working_day', 480)",
        )
        .bind(day_rule_id)
        .bind(version_id)
        .bind(weekday)
        .execute(pool)
        .await
        .expect("insert flex day rule");
        sqlx::query(
            "INSERT INTO work_schedule_work_intervals \
             (day_rule_id, sequence, start_time, start_day_offset, end_time, end_day_offset) \
             VALUES ($1, 1, $2, 0, $3, 0)",
        )
        .bind(day_rule_id)
        .bind(NaiveTime::from_hms_opt(9, 0, 0).expect("start"))
        .bind(NaiveTime::from_hms_opt(18, 0, 0).expect("end"))
        .execute(pool)
        .await
        .expect("insert flex interval");
    }

    if let Some((weekday, start_time, start_offset, end_time, end_offset)) = core_time {
        sqlx::query(
            "INSERT INTO work_schedule_core_time_windows \
             (version_id, weekday, start_time, start_day_offset, end_time, end_day_offset) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(version_id)
        .bind(weekday)
        .bind(start_time)
        .bind(start_offset)
        .bind(end_time)
        .bind(end_offset)
        .execute(pool)
        .await
        .expect("insert core time window");
    }

    sqlx::query(
        "UPDATE work_schedule_versions \
         SET status = 'published', published_by = $2, published_at = $3 \
         WHERE id = $1",
    )
    .bind(version_id)
    .bind(created_by)
    .bind(resolved_at())
    .execute(pool)
    .await
    .expect("publish flex version");

    version_id
}

async fn seed_assignment(
    pool: &PgPool,
    schedule_id: Uuid,
    created_by: &str,
    user_id: Option<&str>,
    department_id: Option<&str>,
    is_org_default: bool,
) -> Uuid {
    seed_assignment_from(
        pool,
        schedule_id,
        created_by,
        user_id,
        department_id,
        is_org_default,
        NaiveDate::from_ymd_opt(2026, 7, 1).expect("valid from"),
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn seed_assignment_from(
    pool: &PgPool,
    schedule_id: Uuid,
    created_by: &str,
    user_id: Option<&str>,
    department_id: Option<&str>,
    is_org_default: bool,
    valid_from: NaiveDate,
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
    .bind(valid_from)
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

fn command_on(user_id: String, work_date: NaiveDate) -> ResolveWorkdayCommand {
    ResolveWorkdayCommand {
        user_id,
        work_date,
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

#[tokio::test]
async fn flex_schedule_snapshots_core_time_window_for_working_day() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_work_schedule_data(&pool).await;
    let actor = seed_user(&pool, UserRole::Manager, true).await;
    let user = seed_user(&pool, UserRole::Employee, false).await;
    let schedule_id = create_schedule(&pool, &actor.id.to_string()).await;
    let version_id = seed_flex_schedule(
        &pool,
        &actor.id.to_string(),
        schedule_id,
        1,
        NaiveDate::from_ymd_opt(2026, 7, 1).expect("effective date"),
        None,
        "follow_weekly_pattern",
        Some((
            1,
            NaiveTime::from_hms_opt(10, 0, 0).expect("core start"),
            0,
            NaiveTime::from_hms_opt(15, 0, 0).expect("core end"),
            0,
        )),
    )
    .await;
    seed_assignment(&pool, schedule_id, &actor.id.to_string(), None, None, true).await;

    let resolved = resolver(&pool)
        .execute(command(user.id.to_string()))
        .await
        .expect("flex resolution");

    assert_eq!(resolved.schedule_type, ScheduleType::Flex);
    assert_eq!(resolved.work_schedule_version_id, version_id.to_string());
    assert_eq!(resolved.core_time_windows.len(), 1);
    let window = &resolved.core_time_windows[0];
    assert_eq!(window.weekday, 1);
    assert_eq!(window.start_time.hour(), 10);
    assert_eq!(window.end_time.hour(), 15);
}

#[tokio::test]
async fn flex_schedule_without_core_time_for_weekday_resolves_empty_windows() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_work_schedule_data(&pool).await;
    let actor = seed_user(&pool, UserRole::Manager, true).await;
    let user = seed_user(&pool, UserRole::Employee, false).await;
    let schedule_id = create_schedule(&pool, &actor.id.to_string()).await;
    seed_flex_schedule(
        &pool,
        &actor.id.to_string(),
        schedule_id,
        1,
        NaiveDate::from_ymd_opt(2026, 7, 1).expect("effective date"),
        None,
        "follow_weekly_pattern",
        Some((
            1,
            NaiveTime::from_hms_opt(10, 0, 0).expect("core start"),
            0,
            NaiveTime::from_hms_opt(15, 0, 0).expect("core end"),
            0,
        )),
    )
    .await;
    seed_assignment(&pool, schedule_id, &actor.id.to_string(), None, None, true).await;

    // Tuesday 2026-07-07 (weekday 2) has no core time window seeded — full flex.
    let resolved = resolver(&pool)
        .execute(command_on(
            user.id.to_string(),
            NaiveDate::from_ymd_opt(2026, 7, 7).expect("tuesday"),
        ))
        .await
        .expect("full-flex resolution");

    assert_eq!(resolved.schedule_type, ScheduleType::Flex);
    assert!(resolved.core_time_windows.is_empty());
}

#[tokio::test]
async fn re_resolution_clears_stale_core_time_window_after_override_to_non_working() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_work_schedule_data(&pool).await;
    let actor = seed_user(&pool, UserRole::Manager, true).await;
    let user = seed_user(&pool, UserRole::Employee, false).await;
    let schedule_id = create_schedule(&pool, &actor.id.to_string()).await;
    seed_flex_schedule(
        &pool,
        &actor.id.to_string(),
        schedule_id,
        1,
        NaiveDate::from_ymd_opt(2026, 7, 1).expect("effective date"),
        None,
        "follow_weekly_pattern",
        Some((
            1,
            NaiveTime::from_hms_opt(10, 0, 0).expect("core start"),
            0,
            NaiveTime::from_hms_opt(15, 0, 0).expect("core end"),
            0,
        )),
    )
    .await;
    seed_assignment(&pool, schedule_id, &actor.id.to_string(), None, None, true).await;

    let first = resolver(&pool)
        .execute(command(user.id.to_string()))
        .await
        .expect("first flex resolution");
    assert_eq!(first.core_time_windows.len(), 1);

    sqlx::query(
        "INSERT INTO workday_overrides \
         (id, user_id, work_date, kind, work_schedule_id, reason, created_by) \
         VALUES ($1, $2, $3, 'non_working_day', NULL, 'test override', $4)",
    )
    .bind(Uuid::new_v4())
    .bind(user.id.to_string())
    .bind(work_date())
    .bind(actor.id.to_string())
    .execute(&pool)
    .await
    .expect("insert non-working override");

    let second = resolver(&pool)
        .execute(command(user.id.to_string()))
        .await
        .expect("second resolution after override");

    assert_eq!(second.id, first.id);
    assert_eq!(second.day_kind, ResolvedDayKind::ScheduledNonWorkingDay);
    assert!(second.core_time_windows.is_empty());

    let window_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM resolved_workday_core_time_windows WHERE resolved_workday_id = $1",
    )
    .bind(Uuid::parse_str(&first.id).expect("resolved id"))
    .fetch_one(&pool)
    .await
    .expect("window count");
    assert_eq!(window_count, 0);
}

#[tokio::test]
async fn locked_flex_projection_core_time_windows_are_immutable() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_work_schedule_data(&pool).await;
    let actor = seed_user(&pool, UserRole::Manager, true).await;
    let user = seed_user(&pool, UserRole::Employee, false).await;
    let schedule_id = create_schedule(&pool, &actor.id.to_string()).await;
    seed_flex_schedule(
        &pool,
        &actor.id.to_string(),
        schedule_id,
        1,
        NaiveDate::from_ymd_opt(2026, 7, 1).expect("effective date"),
        None,
        "follow_weekly_pattern",
        Some((
            1,
            NaiveTime::from_hms_opt(10, 0, 0).expect("core start"),
            0,
            NaiveTime::from_hms_opt(15, 0, 0).expect("core end"),
            0,
        )),
    )
    .await;
    seed_assignment(&pool, schedule_id, &actor.id.to_string(), None, None, true).await;

    let resolved = resolver(&pool)
        .execute(command(user.id.to_string()))
        .await
        .expect("flex resolution");
    let resolved_id = Uuid::parse_str(&resolved.id).expect("resolved id");

    sqlx::query("UPDATE resolved_workdays SET locked_at = $2 WHERE id = $1")
        .bind(resolved_id)
        .bind(resolved_at())
        .execute(&pool)
        .await
        .expect("lock projection");

    let insert_attempt = sqlx::query(
        "INSERT INTO resolved_workday_core_time_windows \
         (resolved_workday_id, weekday, start_time, end_time) \
         VALUES ($1, 2, '08:00:00', '12:00:00')",
    )
    .bind(resolved_id)
    .execute(&pool)
    .await;
    assert!(insert_attempt.is_err());

    let update_attempt = sqlx::query(
        "UPDATE resolved_workday_core_time_windows SET start_time = '09:00:00' \
         WHERE resolved_workday_id = $1",
    )
    .bind(resolved_id)
    .execute(&pool)
    .await;
    assert!(update_attempt.is_err());

    let delete_attempt = sqlx::query(
        "DELETE FROM resolved_workday_core_time_windows WHERE resolved_workday_id = $1",
    )
    .bind(resolved_id)
    .execute(&pool)
    .await;
    assert!(delete_attempt.is_err());
}

#[tokio::test]
async fn version_boundary_snapshots_correct_core_time_window_per_date() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_work_schedule_data(&pool).await;
    let actor = seed_user(&pool, UserRole::Manager, true).await;
    let user = seed_user(&pool, UserRole::Employee, false).await;
    let schedule_id = create_schedule(&pool, &actor.id.to_string()).await;
    let version1 = seed_flex_schedule(
        &pool,
        &actor.id.to_string(),
        schedule_id,
        1,
        NaiveDate::from_ymd_opt(2026, 6, 1).expect("v1 from"),
        Some(NaiveDate::from_ymd_opt(2026, 7, 1).expect("v1 until")),
        "follow_weekly_pattern",
        Some((
            1,
            NaiveTime::from_hms_opt(8, 0, 0).expect("v1 core start"),
            0,
            NaiveTime::from_hms_opt(12, 0, 0).expect("v1 core end"),
            0,
        )),
    )
    .await;
    let version2 = seed_flex_schedule(
        &pool,
        &actor.id.to_string(),
        schedule_id,
        2,
        NaiveDate::from_ymd_opt(2026, 7, 1).expect("v2 from"),
        None,
        "follow_weekly_pattern",
        Some((
            1,
            NaiveTime::from_hms_opt(10, 0, 0).expect("v2 core start"),
            0,
            NaiveTime::from_hms_opt(15, 0, 0).expect("v2 core end"),
            0,
        )),
    )
    .await;
    seed_assignment_from(
        &pool,
        schedule_id,
        &actor.id.to_string(),
        None,
        None,
        true,
        NaiveDate::from_ymd_opt(2026, 6, 1).expect("assignment valid from"),
    )
    .await;

    let before = resolver(&pool)
        .execute(command_on(
            user.id.to_string(),
            NaiveDate::from_ymd_opt(2026, 6, 29).expect("monday before boundary"),
        ))
        .await
        .expect("resolution within version1 range");
    assert_eq!(before.work_schedule_version_id, version1.to_string());
    assert_eq!(before.core_time_windows[0].start_time.hour(), 8);

    let after = resolver(&pool)
        .execute(command(user.id.to_string()))
        .await
        .expect("resolution within version2 range");
    assert_eq!(after.work_schedule_version_id, version2.to_string());
    assert_eq!(after.core_time_windows[0].start_time.hour(), 10);
}

#[tokio::test]
async fn flex_schedule_snapshots_overnight_core_time_window_spanning_week_boundary() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_work_schedule_data(&pool).await;
    let actor = seed_user(&pool, UserRole::Manager, true).await;
    let user = seed_user(&pool, UserRole::Employee, false).await;
    let schedule_id = create_schedule(&pool, &actor.id.to_string()).await;
    seed_flex_schedule(
        &pool,
        &actor.id.to_string(),
        schedule_id,
        1,
        NaiveDate::from_ymd_opt(2026, 7, 1).expect("effective date"),
        None,
        "follow_weekly_pattern",
        Some((
            7,
            NaiveTime::from_hms_opt(23, 0, 0).expect("sunday core start"),
            0,
            NaiveTime::from_hms_opt(2, 0, 0).expect("monday core end"),
            1,
        )),
    )
    .await;
    seed_assignment(&pool, schedule_id, &actor.id.to_string(), None, None, true).await;

    let resolved = resolver(&pool)
        .execute(command_on(
            user.id.to_string(),
            NaiveDate::from_ymd_opt(2026, 7, 5).expect("sunday"),
        ))
        .await
        .expect("sunday flex resolution");

    assert_eq!(resolved.core_time_windows.len(), 1);
    let window = &resolved.core_time_windows[0];
    assert_eq!(window.weekday, 7);
    assert_eq!(window.start_day_offset, 0);
    assert_eq!(window.end_day_offset, 1);
    assert_eq!(window.end_time.hour(), 2);
}

#[tokio::test]
async fn locked_flex_projection_retains_original_version_reference_for_settlement_lookup() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_work_schedule_data(&pool).await;
    let actor = seed_user(&pool, UserRole::Manager, true).await;
    let user = seed_user(&pool, UserRole::Employee, false).await;
    let schedule_id = create_schedule(&pool, &actor.id.to_string()).await;
    let version1 = seed_flex_schedule(
        &pool,
        &actor.id.to_string(),
        schedule_id,
        1,
        NaiveDate::from_ymd_opt(2026, 7, 1).expect("v1 from"),
        Some(NaiveDate::from_ymd_opt(2026, 8, 1).expect("v1 until")),
        "follow_weekly_pattern",
        Some((
            1,
            NaiveTime::from_hms_opt(10, 0, 0).expect("v1 core start"),
            0,
            NaiveTime::from_hms_opt(15, 0, 0).expect("v1 core end"),
            0,
        )),
    )
    .await;
    seed_assignment(&pool, schedule_id, &actor.id.to_string(), None, None, true).await;

    let resolved = resolver(&pool)
        .execute(command(user.id.to_string()))
        .await
        .expect("flex resolution");
    assert_eq!(resolved.work_schedule_version_id, version1.to_string());
    sqlx::query("UPDATE resolved_workdays SET locked_at = $2 WHERE id = $1")
        .bind(Uuid::parse_str(&resolved.id).expect("resolved id"))
        .bind(resolved_at())
        .execute(&pool)
        .await
        .expect("lock projection");

    // A later version is added to the same schedule after the lock — the locked
    // projection must keep referencing version1, not silently pick up version2.
    seed_flex_schedule(
        &pool,
        &actor.id.to_string(),
        schedule_id,
        2,
        NaiveDate::from_ymd_opt(2026, 8, 1).expect("v2 from"),
        None,
        "follow_weekly_pattern",
        Some((
            1,
            NaiveTime::from_hms_opt(9, 0, 0).expect("v2 core start"),
            0,
            NaiveTime::from_hms_opt(11, 0, 0).expect("v2 core end"),
            0,
        )),
    )
    .await;

    let locked = resolver(&pool)
        .execute(command(user.id.to_string()))
        .await
        .expect("locked resolution unaffected by new version");
    assert_eq!(locked.id, resolved.id);
    assert_eq!(locked.work_schedule_version_id, version1.to_string());

    // The original version's settlement/flex reference remains queryable —
    // reproducibility is via the version_id, not a projection-level snapshot.
    let core_time_still_present: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM work_schedule_core_time_windows WHERE version_id = $1",
    )
    .bind(Uuid::parse_str(&version1.to_string()).expect("version1 id"))
    .fetch_one(&pool)
    .await
    .expect("core time count");
    assert_eq!(core_time_still_present, 1);
}

#[tokio::test]
async fn corrupt_fixed_projection_with_core_time_rows_errors_on_read() {
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
        NaiveTime::from_hms_opt(9, 0, 0).expect("start"),
        NaiveTime::from_hms_opt(18, 0, 0).expect("end"),
        0,
    )
    .await;
    seed_assignment(&pool, schedule.id, &actor.id.to_string(), None, None, true).await;
    let resolved = resolver(&pool)
        .execute(command(user.id.to_string()))
        .await
        .expect("fixed resolution");
    assert_eq!(resolved.schedule_type, ScheduleType::Fixed);

    // Simulate corrupt data: a core-time row attached to a fixed-schedule projection.
    // This bypasses the application layer (which never writes such a row) to prove
    // the readback guard in `assemble_resolved_workday` actually fires.
    sqlx::query(
        "INSERT INTO resolved_workday_core_time_windows \
         (resolved_workday_id, weekday, start_time, end_time) \
         VALUES ($1, 1, '10:00:00', '15:00:00')",
    )
    .bind(Uuid::parse_str(&resolved.id).expect("resolved id"))
    .execute(&pool)
    .await
    .expect("insert corrupt core time row");

    let error = resolver(&pool)
        .repository()
        .find_resolved(&user.id.to_string(), work_date())
        .await
        .expect_err("corrupt fixed projection with core time must error on read");
    assert!(matches!(error, ResolveWorkdayError::InvalidScheduleData(_)));
}

#[tokio::test]
async fn corrupt_non_workday_projection_with_core_time_rows_errors_on_read() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_work_schedule_data(&pool).await;
    let actor = seed_user(&pool, UserRole::Manager, true).await;
    let user = seed_user(&pool, UserRole::Employee, false).await;
    let schedule_id = create_schedule(&pool, &actor.id.to_string()).await;
    seed_flex_schedule(
        &pool,
        &actor.id.to_string(),
        schedule_id,
        1,
        NaiveDate::from_ymd_opt(2026, 7, 1).expect("effective date"),
        None,
        "follow_weekly_pattern",
        Some((
            1,
            NaiveTime::from_hms_opt(10, 0, 0).expect("core start"),
            0,
            NaiveTime::from_hms_opt(15, 0, 0).expect("core end"),
            0,
        )),
    )
    .await;
    seed_assignment(&pool, schedule_id, &actor.id.to_string(), None, None, true).await;
    sqlx::query(
        "INSERT INTO workday_overrides \
         (id, user_id, work_date, kind, work_schedule_id, reason, created_by) \
         VALUES ($1, $2, $3, 'non_working_day', NULL, 'test override', $4)",
    )
    .bind(Uuid::new_v4())
    .bind(user.id.to_string())
    .bind(work_date())
    .bind(actor.id.to_string())
    .execute(&pool)
    .await
    .expect("insert non-working override");

    let resolved = resolver(&pool)
        .execute(command(user.id.to_string()))
        .await
        .expect("non-working resolution");
    assert_eq!(resolved.day_kind, ResolvedDayKind::ScheduledNonWorkingDay);
    assert!(resolved.core_time_windows.is_empty());

    // Simulate corrupt data: a core-time row attached to a non-workday projection.
    sqlx::query(
        "INSERT INTO resolved_workday_core_time_windows \
         (resolved_workday_id, weekday, start_time, end_time) \
         VALUES ($1, 1, '10:00:00', '15:00:00')",
    )
    .bind(Uuid::parse_str(&resolved.id).expect("resolved id"))
    .execute(&pool)
    .await
    .expect("insert corrupt core time row");

    let error = resolver(&pool)
        .repository()
        .find_resolved(&user.id.to_string(), work_date())
        .await
        .expect_err("corrupt non-workday projection with core time must error on read");
    assert!(matches!(error, ResolveWorkdayError::InvalidScheduleData(_)));
}

#[tokio::test]
async fn re_resolution_from_flex_to_fixed_schedule_clears_core_time_window() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_work_schedule_data(&pool).await;
    let actor = seed_user(&pool, UserRole::Manager, true).await;
    let user = seed_user(&pool, UserRole::Employee, false).await;
    let flex_schedule_id = create_schedule(&pool, &actor.id.to_string()).await;
    seed_flex_schedule(
        &pool,
        &actor.id.to_string(),
        flex_schedule_id,
        1,
        NaiveDate::from_ymd_opt(2026, 7, 1).expect("effective date"),
        None,
        "follow_weekly_pattern",
        Some((
            1,
            NaiveTime::from_hms_opt(10, 0, 0).expect("core start"),
            0,
            NaiveTime::from_hms_opt(15, 0, 0).expect("core end"),
            0,
        )),
    )
    .await;
    seed_assignment(
        &pool,
        flex_schedule_id,
        &actor.id.to_string(),
        None,
        None,
        true,
    )
    .await;

    let first = resolver(&pool)
        .execute(command(user.id.to_string()))
        .await
        .expect("first flex resolution");
    assert_eq!(first.schedule_type, ScheduleType::Flex);
    assert_eq!(first.core_time_windows.len(), 1);

    // A higher-priority user-level assignment now points at a fixed schedule
    // covering the same work_date. Re-resolution must replace the row in place
    // and clear the stale flex core-time snapshot, not merely leave it stale.
    let fixed_schedule = seed_schedule(
        &pool,
        &actor.id.to_string(),
        "follow_weekly_pattern",
        NaiveTime::from_hms_opt(9, 0, 0).expect("start"),
        NaiveTime::from_hms_opt(18, 0, 0).expect("end"),
        0,
    )
    .await;
    seed_assignment(
        &pool,
        fixed_schedule.id,
        &actor.id.to_string(),
        Some(&user.id.to_string()),
        None,
        false,
    )
    .await;

    let second = resolver(&pool)
        .execute(command(user.id.to_string()))
        .await
        .expect("second resolution after user assignment change");

    assert_eq!(second.id, first.id);
    assert_eq!(second.schedule_type, ScheduleType::Fixed);
    assert!(second.core_time_windows.is_empty());
    let window_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM resolved_workday_core_time_windows WHERE resolved_workday_id = $1",
    )
    .bind(Uuid::parse_str(&first.id).expect("resolved id"))
    .fetch_one(&pool)
    .await
    .expect("window count");
    assert_eq!(window_count, 0);
}

#[tokio::test]
async fn re_resolution_to_a_different_flex_schedule_without_core_time_clears_stale_window() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_work_schedule_data(&pool).await;
    let actor = seed_user(&pool, UserRole::Manager, true).await;
    let user = seed_user(&pool, UserRole::Employee, false).await;
    let schedule_with_core_id = create_schedule(&pool, &actor.id.to_string()).await;
    seed_flex_schedule(
        &pool,
        &actor.id.to_string(),
        schedule_with_core_id,
        1,
        NaiveDate::from_ymd_opt(2026, 7, 1).expect("effective date"),
        None,
        "follow_weekly_pattern",
        Some((
            1,
            NaiveTime::from_hms_opt(10, 0, 0).expect("core start"),
            0,
            NaiveTime::from_hms_opt(15, 0, 0).expect("core end"),
            0,
        )),
    )
    .await;
    seed_assignment(
        &pool,
        schedule_with_core_id,
        &actor.id.to_string(),
        None,
        None,
        true,
    )
    .await;

    let first = resolver(&pool)
        .execute(command(user.id.to_string()))
        .await
        .expect("first flex resolution");
    assert_eq!(first.schedule_type, ScheduleType::Flex);
    assert_eq!(first.core_time_windows.len(), 1);

    // Published versions are immutable, so an already-published version's core
    // time cannot change out from under a date it already covers. What CAN
    // change is which schedule/version applies: a higher-priority user-level
    // assignment now points at a different flex schedule with no core time for
    // this weekday (full flex). schedule_type stays "flex" on both sides, but
    // the stale window must still be cleared, not merged or left behind.
    let full_flex_schedule_id = create_schedule(&pool, &actor.id.to_string()).await;
    seed_flex_schedule(
        &pool,
        &actor.id.to_string(),
        full_flex_schedule_id,
        1,
        NaiveDate::from_ymd_opt(2026, 7, 1).expect("effective date"),
        None,
        "follow_weekly_pattern",
        None,
    )
    .await;
    seed_assignment(
        &pool,
        full_flex_schedule_id,
        &actor.id.to_string(),
        Some(&user.id.to_string()),
        None,
        false,
    )
    .await;

    let second = resolver(&pool)
        .execute(command(user.id.to_string()))
        .await
        .expect("second resolution against full-flex schedule");

    assert_eq!(second.id, first.id);
    assert_eq!(second.schedule_type, ScheduleType::Flex);
    assert_ne!(
        second.work_schedule_version_id,
        first.work_schedule_version_id
    );
    assert!(second.core_time_windows.is_empty());
    let window_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM resolved_workday_core_time_windows WHERE resolved_workday_id = $1",
    )
    .bind(Uuid::parse_str(&first.id).expect("resolved id"))
    .fetch_one(&pool)
    .await
    .expect("window count");
    assert_eq!(window_count, 0);
}
