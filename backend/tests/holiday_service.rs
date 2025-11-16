use chrono::{Duration, NaiveDate};
use sqlx::PgPool;
use timekeeper_backend::services::holiday::{HolidayReason, HolidayService};
use uuid::Uuid;

async fn seed_test_user(pool: &PgPool) -> sqlx::Result<String> {
    let user_id = Uuid::new_v4().to_string();
    let username_prefix: String = user_id.chars().take(8).collect();

    sqlx::query(
        "INSERT INTO users (id, username, password_hash, full_name, role, is_system_admin, \
         mfa_secret, mfa_enabled_at, created_at, updated_at) \
         VALUES ($1, $2, 'hash', 'User', 'employee', FALSE, NULL, NULL, NOW(), NOW())",
    )
    .bind(&user_id)
    .bind(format!("user_{}", username_prefix))
    .execute(pool)
    .await?;

    Ok(user_id)
}

#[sqlx::test(migrations = "./migrations")]
async fn detects_public_holiday(pool: PgPool) -> sqlx::Result<()> {
    let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    sqlx::query(
        "INSERT INTO holidays (id, holiday_date, name, description, created_at, updated_at) \
         VALUES ($1, $2, 'New Year', NULL, NOW(), NOW())",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(date)
    .execute(&pool)
    .await?;

    let service = HolidayService::new(pool.clone());
    let decision = service.is_holiday(date, None).await?;

    assert!(decision.is_holiday);
    assert_eq!(decision.reason, HolidayReason::PublicHoliday);
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn detects_weekly_holiday(pool: PgPool) -> sqlx::Result<()> {
    let wed = NaiveDate::from_ymd_opt(2025, 1, 8).unwrap(); // Wednesday

    sqlx::query(
        "INSERT INTO weekly_holidays \
            (id, weekday, starts_on, ends_on, enforced_from, enforced_to, created_by, created_at) \
         VALUES ($1, 2, $2, NULL, $2, NULL, 'system', NOW())",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(wed - Duration::days(7))
    .execute(&pool)
    .await?;

    let service = HolidayService::new(pool.clone());
    let decision = service.is_holiday(wed, None).await?;

    assert!(decision.is_holiday);
    assert_eq!(decision.reason, HolidayReason::WeeklyHoliday);
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn exception_can_override_weekly_holiday(pool: PgPool) -> sqlx::Result<()> {
    let wed = NaiveDate::from_ymd_opt(2025, 1, 8).unwrap();
    let user_id = seed_test_user(&pool).await?;

    sqlx::query(
        "INSERT INTO weekly_holidays \
            (id, weekday, starts_on, ends_on, enforced_from, enforced_to, created_by, created_at) \
         VALUES ($1, 2, $2, NULL, $2, NULL, 'system', NOW())",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(wed - Duration::days(7))
    .execute(&pool)
    .await?;

    sqlx::query(
        "INSERT INTO holiday_exceptions \
            (id, user_id, exception_date, override, reason, created_by, created_at) \
         VALUES ($1, $2, $3, FALSE, 'Override to work', 'system', NOW())",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&user_id)
    .bind(wed)
    .execute(&pool)
    .await?;

    let service = HolidayService::new(pool.clone());
    let decision = service.is_holiday(wed, Some(&user_id)).await?;

    assert!(!decision.is_holiday);
    assert_eq!(decision.reason, HolidayReason::ExceptionOverride);
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn monthly_listing_combines_sources(pool: PgPool) -> sqlx::Result<()> {
    let jan1 = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    sqlx::query(
        "INSERT INTO holidays (id, holiday_date, name, description, created_at, updated_at) \
         VALUES ($1, $2, 'New Year', NULL, NOW(), NOW())",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(jan1)
    .execute(&pool)
    .await?;

    let wed = NaiveDate::from_ymd_opt(2025, 1, 8).unwrap();
    sqlx::query(
        "INSERT INTO weekly_holidays \
            (id, weekday, starts_on, ends_on, enforced_from, enforced_to, created_by, created_at, updated_at) \
         VALUES ($1, 2, $2, NULL, $2, NULL, 'system', NOW(), NOW())",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(wed - Duration::days(7))
    .execute(&pool)
    .await?;

    let service = HolidayService::new(pool.clone());
    let entries = service.list_month(2025, 1, None).await?;

    assert!(entries
        .iter()
        .any(|entry| entry.date == jan1 && entry.reason == HolidayReason::PublicHoliday));
    assert!(entries
        .iter()
        .any(|entry| entry.date == wed && entry.reason == HolidayReason::WeeklyHoliday));

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn monthly_listing_includes_exception_overrides(pool: PgPool) -> sqlx::Result<()> {
    let target = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();
    let user_id = seed_test_user(&pool).await?;

    sqlx::query(
        "INSERT INTO holiday_exceptions \
            (id, user_id, exception_date, override, reason, created_by, created_at) \
         VALUES ($1, $2, $3, TRUE, 'Personal leave', 'system', NOW())",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&user_id)
    .bind(target)
    .execute(&pool)
    .await?;

    let service = HolidayService::new(pool.clone());
    let entries = service.list_month(2025, 1, Some(&user_id)).await?;

    assert!(entries
        .iter()
        .any(|entry| { entry.date == target && entry.reason == HolidayReason::ExceptionOverride }));

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn monthly_listing_respects_weekly_effective_range(pool: PgPool) -> sqlx::Result<()> {
    let enforced_from = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();
    let enforced_to = NaiveDate::from_ymd_opt(2025, 1, 20).unwrap();

    sqlx::query(
        "INSERT INTO weekly_holidays \
            (id, weekday, starts_on, ends_on, enforced_from, enforced_to, created_by, created_at, updated_at) \
         VALUES ($1, 4, $2, $3, $2, $3, 'system', NOW(), NOW())",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(enforced_from)
    .bind(enforced_to)
    .execute(&pool)
    .await?;

    let service = HolidayService::new(pool.clone());
    let entries = service.list_month(2025, 1, None).await?;

    let weekly_dates: Vec<_> = entries
        .iter()
        .filter(|entry| entry.reason == HolidayReason::WeeklyHoliday)
        .map(|entry| entry.date)
        .collect();

    assert_eq!(
        weekly_dates,
        vec![enforced_from, enforced_from + Duration::days(7)]
    );

    Ok(())
}
