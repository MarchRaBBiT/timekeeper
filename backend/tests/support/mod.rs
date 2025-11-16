use chrono::{Datelike, Duration, NaiveDate};
use chrono_tz::Asia::Tokyo;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{
    env,
    sync::{Arc, OnceLock},
    time::Duration as StdDuration,
};
use timekeeper_backend::{
    config::Config,
    models::user::{User, UserRole},
    models::{
        holiday::Holiday,
        leave_request::{LeaveRequest, LeaveType},
        overtime_request::OvertimeRequest,
    },
};
use tokio::{
    sync::{Mutex, OwnedMutexGuard},
    time::timeout,
};
use uuid::Uuid;

const DEFAULT_DATABASE_URL: &str = "postgres://timekeeper:timekeeper@localhost:5432/timekeeper";

pub fn test_config() -> Config {
    let database_url = database_url();

    Config {
        database_url,
        jwt_secret: "a_secure_token_that_is_long_enough_123".into(),
        jwt_expiration_hours: 1,
        refresh_token_expiration_days: 7,
        time_zone: Tokyo,
        mfa_issuer: "Timekeeper".into(),
    }
}

fn database_url() -> String {
    env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.into())
}

pub struct TestDatabase {
    pool: PgPool,
    _guard: OwnedMutexGuard<()>,
}

impl TestDatabase {
    pub fn clone_pool(&self) -> PgPool {
        self.pool.clone()
    }
}

impl std::ops::Deref for TestDatabase {
    type Target = PgPool;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}

impl AsRef<PgPool> for TestDatabase {
    fn as_ref(&self) -> &PgPool {
        &self.pool
    }
}

static TEST_DB_GUARD: OnceLock<Arc<Mutex<()>>> = OnceLock::new();

fn test_db_guard() -> Arc<Mutex<()>> {
    TEST_DB_GUARD
        .get_or_init(|| Arc::new(Mutex::new(())))
        .clone()
}

pub async fn setup_test_pool() -> Option<TestDatabase> {
    let database_url = database_url();
    let connect_future = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(StdDuration::from_secs(5))
        .connect(&database_url);

    let pool = match timeout(StdDuration::from_secs(3), connect_future).await {
        Ok(Ok(pool)) => pool,
        Ok(Err(error)) => {
            eprintln!("Skipping DB-backed test: {error}");
            return None;
        }
        Err(_) => {
            eprintln!("Skipping DB-backed test: timed out connecting to {database_url}");
            return None;
        }
    };

    let guard = test_db_guard().lock_owned().await;

    if let Err(error) = sqlx::migrate!("./migrations").run(&pool).await {
        eprintln!("Skipping DB-backed test (migration failed): {error}");
        return None;
    }

    if let Err(error) = reset_database(&pool).await {
        eprintln!("Skipping DB-backed test (cleanup failed): {error}");
        return None;
    }

    Some(TestDatabase {
        pool,
        _guard: guard,
    })
}

async fn reset_database(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "TRUNCATE TABLE holiday_exceptions, weekly_holidays, holidays, leave_requests, overtime_requests, \
         break_records, attendance, refresh_tokens, users RESTART IDENTITY CASCADE",
    )
    .execute(pool)
    .await?;

    Ok(())
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

pub async fn seed_leave_request(
    pool: &PgPool,
    user_id: &str,
    leave_type: LeaveType,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> LeaveRequest {
    let request = LeaveRequest::new(
        user_id.to_string(),
        leave_type,
        start_date,
        end_date,
        Some("test".into()),
    );
    sqlx::query(
        "INSERT INTO leave_requests (id, user_id, leave_type, start_date, end_date, reason, status, approved_by, approved_at, decision_comment, rejected_by, rejected_at, cancelled_at, created_at, updated_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)",
    )
    .bind(&request.id)
    .bind(&request.user_id)
    .bind(match request.leave_type {
        LeaveType::Annual => "annual",
        LeaveType::Sick => "sick",
        LeaveType::Personal => "personal",
        LeaveType::Other => "other",
    })
    .bind(&request.start_date)
    .bind(&request.end_date)
    .bind(&request.reason)
    .bind(match request.status {
        timekeeper_backend::models::leave_request::RequestStatus::Pending => "pending",
        timekeeper_backend::models::leave_request::RequestStatus::Approved => "approved",
        timekeeper_backend::models::leave_request::RequestStatus::Rejected => "rejected",
        timekeeper_backend::models::leave_request::RequestStatus::Cancelled => "cancelled",
    })
    .bind(&request.approved_by)
    .bind(&request.approved_at)
    .bind(&request.decision_comment)
    .bind(&request.rejected_by)
    .bind(&request.rejected_at)
    .bind(&request.cancelled_at)
    .bind(&request.created_at)
    .bind(&request.updated_at)
    .execute(pool)
    .await
    .expect("insert leave request");
    request
}

pub async fn seed_overtime_request(
    pool: &PgPool,
    user_id: &str,
    date: NaiveDate,
    planned_hours: f64,
) -> OvertimeRequest {
    let request = OvertimeRequest::new(
        user_id.to_string(),
        date,
        planned_hours,
        Some("test OT".into()),
    );
    sqlx::query(
        "INSERT INTO overtime_requests (id, user_id, date, planned_hours, reason, status, approved_by, approved_at, decision_comment, rejected_by, rejected_at, cancelled_at, created_at, updated_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
    )
    .bind(&request.id)
    .bind(&request.user_id)
    .bind(&request.date)
    .bind(&request.planned_hours)
    .bind(&request.reason)
    .bind(match request.status {
        timekeeper_backend::models::overtime_request::RequestStatus::Pending => "pending",
        timekeeper_backend::models::overtime_request::RequestStatus::Approved => "approved",
        timekeeper_backend::models::overtime_request::RequestStatus::Rejected => "rejected",
        timekeeper_backend::models::overtime_request::RequestStatus::Cancelled => "cancelled",
    })
    .bind(&request.approved_by)
    .bind(&request.approved_at)
    .bind(&request.decision_comment)
    .bind(&request.rejected_by)
    .bind(&request.rejected_at)
    .bind(&request.cancelled_at)
    .bind(&request.created_at)
    .bind(&request.updated_at)
    .execute(pool)
    .await
    .expect("insert overtime request");
    request
}

pub async fn seed_public_holiday(pool: &PgPool, date: NaiveDate, name: &str) -> Holiday {
    let holiday = Holiday::new(date, name.to_string(), None);
    sqlx::query(
        "INSERT INTO holidays (id, holiday_date, name, description, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(&holiday.id)
    .bind(&holiday.holiday_date)
    .bind(&holiday.name)
    .bind(&holiday.description)
    .bind(&holiday.created_at)
    .bind(&holiday.updated_at)
    .execute(pool)
    .await
    .expect("insert holiday");
    holiday
}

pub async fn seed_holiday_exception(
    pool: &PgPool,
    user_id: &str,
    date: NaiveDate,
    override_value: bool,
    reason: &str,
) {
    sqlx::query(
        "INSERT INTO holiday_exceptions \
            (id, user_id, exception_date, override, reason, created_by, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, 'test', NOW(), NOW())",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(user_id)
    .bind(date)
    .bind(override_value)
    .bind(reason)
    .execute(pool)
    .await
    .expect("insert holiday exception");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        static ENV_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
        ENV_MUTEX
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("lock env")
    }

    fn restore_env(original: Option<String>) {
        if let Some(value) = original {
            env::set_var("DATABASE_URL", value);
        } else {
            env::remove_var("DATABASE_URL");
        }
    }

    #[test]
    fn test_config_uses_database_url_from_env() {
        let _guard = env_guard();
        let original = env::var("DATABASE_URL").ok();
        env::set_var("DATABASE_URL", "postgres://override/testdb");

        let config = test_config();

        assert_eq!(config.database_url, "postgres://override/testdb");
        restore_env(original);
    }

    #[test]
    fn test_config_falls_back_to_default_when_env_missing() {
        let _guard = env_guard();
        let original = env::var("DATABASE_URL").ok();
        env::remove_var("DATABASE_URL");

        let config = test_config();

        assert_eq!(
            config.database_url,
            "postgres://timekeeper:timekeeper@localhost:5432/timekeeper"
        );
        restore_env(original);
    }
}
