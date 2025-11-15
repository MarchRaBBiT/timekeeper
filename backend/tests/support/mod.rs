use chrono::{Datelike, Duration, NaiveDate};
use chrono_tz::Asia::Tokyo;
use sqlx::PgPool;
use timekeeper_backend::{
    config::Config,
    models::user::{User, UserRole},
};
use uuid::Uuid;

pub fn test_config() -> Config {
    Config {
        database_url: "postgres://timekeeper:timekeeper@localhost:5432/timekeeper".into(),
        jwt_secret: "a_secure_token_that_is_long_enough_123".into(),
        jwt_expiration_hours: 1,
        refresh_token_expiration_days: 7,
        time_zone: Tokyo,
        mfa_issuer: "Timekeeper".into(),
    }
}

pub async fn seed_user(pool: &PgPool, role: UserRole, is_system_admin: bool) -> User {
    let user = User::new(
        format!("user_{}", Uuid::new_v4().to_string()),
        "hash".into(),
        "Test User".into(),
        role,
        is_system_admin,
    );

    sqlx::query(
        "INSERT INTO users (id, username, password_hash, full_name, role, is_system_admin, \
         mfa_secret, mfa_enabled_at, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(&user.id)
    .bind(&user.username)
    .bind(&user.password_hash)
    .bind(&user.full_name)
    .bind(user.role.as_str())
    .bind(&user.is_system_admin)
    .bind(&user.mfa_secret)
    .bind(&user.mfa_enabled_at)
    .bind(&user.created_at)
    .bind(&user.updated_at)
    .execute(pool)
    .await
    .expect("insert user");

    user
}

pub async fn seed_weekly_holiday(pool: &PgPool, date: NaiveDate) {
    let weekday = date.weekday().num_days_from_monday() as i16;
    sqlx::query(
        "INSERT INTO weekly_holidays \
            (id, weekday, starts_on, ends_on, enforced_from, enforced_to, created_by, created_at, updated_at) \
         VALUES ($1, $2, $3, NULL, $4, NULL, 'test', NOW(), NOW())",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(weekday)
    .bind(date - Duration::days(7))
    .bind(date)
    .execute(pool)
    .await
    .expect("insert weekly holiday");
}
