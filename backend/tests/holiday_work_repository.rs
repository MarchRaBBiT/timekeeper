use chrono::{NaiveDate, NaiveTime, Utc};
use sqlx::PgPool;
use timekeeper_backend::{
    models::user::UserRole,
    repositories::holiday_work::{HolidayWorkRepository, HolidayWorkRepositoryError},
};
use timekeeper_contract::holiday_work::{HolidayWorkBenefit, SubmitHolidayWorkRequest};
use timekeeper_domain::{
    attendance_classification::{
        classify_days, ActualWorkInterval, ClassificationDayInput, WorkRuleParameters,
    },
    work_schedules::{ResolvedDayKind, ScheduleType},
};
use uuid::Uuid;

mod support;
use support::{integration_guard, seed_user, seed_work_schedule_for_user, test_pool};

fn date(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 8, day).expect("date")
}

async fn resolved(
    pool: &PgPool,
    user_id: &str,
    schedule_id: Uuid,
    version_id: Uuid,
    work_date: NaiveDate,
    day_kind: &str,
    locked: bool,
) {
    sqlx::query(
        "INSERT INTO resolved_workdays
         (id,user_id,work_date,work_schedule_id,work_schedule_version_id,source,source_id,
          day_kind,timezone,workday_boundary,expected_work_minutes,resolved_at,locked_at)
         VALUES ($1,$2,$3,$4,$5,'user',$6,$7,'Asia/Tokyo',$8,480,NOW(),$9)",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(work_date)
    .bind(schedule_id)
    .bind(version_id)
    .bind(Uuid::new_v4())
    .bind(day_kind)
    .bind(NaiveTime::from_hms_opt(5, 0, 0).expect("time"))
    .bind(locked.then(Utc::now))
    .execute(pool)
    .await
    .expect("resolved");
}

#[tokio::test]
async fn substitution_creates_both_overrides_atomically() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let manager = seed_user(&pool, UserRole::Manager, true).await;
    let (schedule, version) = seed_work_schedule_for_user(&pool, employee.id, "non_working").await;
    resolved(
        &pool,
        &employee.id.to_string(),
        schedule,
        version,
        date(9),
        "scheduled_non_working_day",
        false,
    )
    .await;
    resolved(
        &pool,
        &employee.id.to_string(),
        schedule,
        version,
        date(10),
        "scheduled_workday",
        false,
    )
    .await;
    let repository = HolidayWorkRepository::new(pool.clone());
    let request = repository
        .submit(
            &employee.id.to_string(),
            SubmitHolidayWorkRequest {
                work_date: date(9),
                benefit: HolidayWorkBenefit::Substitution,
                substitute_date: Some(date(10)),
                compensatory_minutes: None,
                reason: "swap".into(),
            },
        )
        .await
        .expect("submit");
    repository
        .approve(
            Uuid::parse_str(&request.id).expect("id"),
            &manager.id.to_string(),
            true,
            "ok",
        )
        .await
        .expect("approve");
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workday_overrides WHERE user_id = $1
         AND work_date = ANY($2)",
    )
    .bind(employee.id.to_string())
    .bind(vec![date(9), date(10)])
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(count, 2);
    let classified = classify_days(&[ClassificationDayInput {
        work_date: date(9),
        day_kind: ResolvedDayKind::ScheduledWorkday,
        schedule_type: ScheduleType::Fixed,
        expected_work_minutes: 480,
        intervals: vec![ActualWorkInterval {
            start: date(9).and_hms_opt(9, 0, 0).unwrap(),
            end: date(9).and_hms_opt(17, 0, 0).unwrap(),
        }],
        rules: WorkRuleParameters {
            statutory_daily_minutes: 480,
            statutory_weekly_minutes: 2400,
            night_start: NaiveTime::from_hms_opt(22, 0, 0).unwrap(),
            night_end: NaiveTime::from_hms_opt(5, 0, 0).unwrap(),
            week_start_weekday: 1,
            legal_holiday_weekday: 7,
        },
    }]);
    assert_eq!(classified[0].legal_holiday_minutes, 0);
    repository
        .cancel(
            Uuid::parse_str(&request.id).expect("id"),
            &employee.id.to_string(),
        )
        .await
        .expect("cancel");
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM workday_overrides WHERE user_id = $1")
            .bind(employee.id.to_string())
            .fetch_one(&pool)
            .await
            .expect("count");
    assert_eq!(count, 0);
}

#[tokio::test]
async fn locked_substitute_rolls_back_the_pair() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let manager = seed_user(&pool, UserRole::Manager, true).await;
    let (schedule, version) = seed_work_schedule_for_user(&pool, employee.id, "non_working").await;
    resolved(
        &pool,
        &employee.id.to_string(),
        schedule,
        version,
        date(16),
        "scheduled_non_working_day",
        false,
    )
    .await;
    resolved(
        &pool,
        &employee.id.to_string(),
        schedule,
        version,
        date(17),
        "scheduled_workday",
        true,
    )
    .await;
    let repository = HolidayWorkRepository::new(pool.clone());
    let request = repository
        .submit(
            &employee.id.to_string(),
            SubmitHolidayWorkRequest {
                work_date: date(16),
                benefit: HolidayWorkBenefit::Substitution,
                substitute_date: Some(date(17)),
                compensatory_minutes: None,
                reason: "locked".into(),
            },
        )
        .await
        .expect("submit");
    let error = repository
        .approve(
            Uuid::parse_str(&request.id).expect("id"),
            &manager.id.to_string(),
            true,
            "ok",
        )
        .await
        .expect_err("locked");
    assert!(matches!(error, HolidayWorkRepositoryError::Locked));
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM workday_overrides WHERE user_id = $1")
            .bind(employee.id.to_string())
            .fetch_one(&pool)
            .await
            .expect("count");
    assert_eq!(count, 0);
}

#[tokio::test]
async fn compensatory_approval_grants_one_expiring_lot() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let manager = seed_user(&pool, UserRole::Manager, true).await;
    sqlx::query(
        "INSERT INTO compensatory_leave_settings
         (id,effective_from,expiry_months,created_by) VALUES ($1,$2,2,$3)",
    )
    .bind(Uuid::new_v4())
    .bind(date(1))
    .bind(manager.id.to_string())
    .execute(&pool)
    .await
    .expect("setting");
    let repository = HolidayWorkRepository::new(pool.clone());
    let request = repository
        .submit(
            &employee.id.to_string(),
            SubmitHolidayWorkRequest {
                work_date: date(23),
                benefit: HolidayWorkBenefit::Compensatory,
                substitute_date: None,
                compensatory_minutes: Some(480),
                reason: "comp".into(),
            },
        )
        .await
        .expect("submit");
    repository
        .approve(
            Uuid::parse_str(&request.id).expect("id"),
            &manager.id.to_string(),
            true,
            "ok",
        )
        .await
        .expect("approve");
    let grant: (i32, NaiveDate, i32) = sqlx::query_as(
        "SELECT amount_minutes, expires_at, obligation_minutes
         FROM leave_ledger_entries WHERE holiday_work_request_id = $1",
    )
    .bind(Uuid::parse_str(&request.id).expect("id"))
    .fetch_one(&pool)
    .await
    .expect("grant");
    assert_eq!(
        grant,
        (480, NaiveDate::from_ymd_opt(2026, 10, 23).unwrap(), 0)
    );
    repository
        .cancel(
            Uuid::parse_str(&request.id).expect("id"),
            &employee.id.to_string(),
        )
        .await
        .expect("cancel");
    let remaining: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_minutes), 0)::BIGINT FROM leave_ledger_entries
         WHERE lot_id = (SELECT lot_id FROM leave_ledger_entries
                         WHERE holiday_work_request_id = $1)",
    )
    .bind(Uuid::parse_str(&request.id).expect("id"))
    .fetch_one(&pool)
    .await
    .expect("remaining");
    assert_eq!(remaining, 0);
}
