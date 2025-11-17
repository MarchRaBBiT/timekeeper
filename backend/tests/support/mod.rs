use chrono::{Datelike, Duration as ChronoDuration, NaiveDate};
use chrono_tz::Asia::Tokyo;
use ctor::ctor;
use pg_embed::{
    pg_enums::PgAuthMethod,
    pg_fetch::{PgFetchSettings, PG_V15},
    postgres::{PgEmbed, PgSettings},
};
use sqlx::PgPool;
use std::{env, net::TcpListener, path::PathBuf, sync::OnceLock, time::Duration as StdDuration};
use tempfile::TempDir;
use timekeeper_backend::{
    config::Config,
    models::user::{User, UserRole},
    models::{
        holiday::Holiday,
        leave_request::{LeaveRequest, LeaveType},
        overtime_request::OvertimeRequest,
    },
    utils::password::hash_password,
};
use uuid::Uuid;

struct EmbeddedPostgres {
    #[allow(dead_code)]
    instance: PgEmbed,
    #[allow(dead_code)]
    data_dir: TempDir,
}

impl std::fmt::Debug for EmbeddedPostgres {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmbeddedPostgres").finish()
    }
}

static EMBEDDED_PG: OnceLock<EmbeddedPostgres> = OnceLock::new();
static EMBEDDED_DB_URL: OnceLock<String> = OnceLock::new();

#[ctor]
fn init_test_database_url() {
    if env::var("DATABASE_URL").is_ok() || env::var("TEST_DATABASE_URL").is_ok() {
        return;
    }
    let url = start_embedded_postgres();
    env::set_var("DATABASE_URL", url.clone());
    env::set_var("TEST_DATABASE_URL", url);
}

fn start_embedded_postgres() -> String {
    let url = EMBEDDED_DB_URL.get().cloned().unwrap_or_else(|| {
        let data_dir = tempfile::tempdir().expect("create temp dir for embedded postgres");
        let db_path: PathBuf = data_dir.path().join("data");
        let pg_settings = PgSettings {
            database_dir: db_path,
            port: allocate_ephemeral_port(),
            user: "timekeeper_test".into(),
            password: "timekeeper_test".into(),
            auth_method: PgAuthMethod::Plain,
            persistent: false,
            timeout: Some(StdDuration::from_secs(15)),
            migration_dir: None,
        };
        let fetch_settings = PgFetchSettings {
            version: PG_V15,
            ..Default::default()
        };

        let runtime = tokio::runtime::Runtime::new().expect("create tokio runtime");
        let mut pg_embed: Option<PgEmbed> = None;
        runtime.block_on(async {
            let mut pg = PgEmbed::new(pg_settings, fetch_settings)
                .await
                .expect("init embedded postgres");
            pg.setup().await.expect("setup embedded postgres");
            pg.start_db().await.expect("start embedded postgres");
            pg_embed = Some(pg);
        });
        let pg = pg_embed.expect("embedded postgres initialized");
        let url = format!(
            "postgres://{}:{}@127.0.0.1:{}/postgres",
            pg.pg_settings.user, pg.pg_settings.password, pg.pg_settings.port
        );
        EMBEDDED_PG
            .set(EmbeddedPostgres {
                instance: pg,
                data_dir,
            })
            .expect("set embedded postgres instance");
        EMBEDDED_DB_URL
            .set(url.clone())
            .expect("set embedded postgres url");
        url
    });
    env::set_var("DATABASE_URL", url.clone());
    env::set_var("TEST_DATABASE_URL", url.clone());
    url
}

fn allocate_ephemeral_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind ephemeral port")
        .local_addr()
        .expect("read socket addr")
        .port()
}

pub fn test_config() -> Config {
    let database_url = test_database_url();

    Config {
        database_url,
        jwt_secret: "a_secure_token_that_is_long_enough_123".into(),
        jwt_expiration_hours: 1,
        refresh_token_expiration_days: 7,
        time_zone: Tokyo,
        mfa_issuer: "Timekeeper".into(),
    }
}

fn test_database_url() -> String {
    env::var("TEST_DATABASE_URL")
        .or_else(|_| env::var("DATABASE_URL"))
        .unwrap_or_else(|_| start_embedded_postgres())
}

async fn insert_user_with_password_hash(
    pool: &PgPool,
    role: UserRole,
    is_system_admin: bool,
    password_hash: String,
) -> User {
    let user = User::new(
        format!("user_{}", Uuid::new_v4().to_string()),
        password_hash,
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

pub async fn seed_user(pool: &PgPool, role: UserRole, is_system_admin: bool) -> User {
    insert_user_with_password_hash(pool, role, is_system_admin, "hash".into()).await
}

pub async fn seed_user_with_password(
    pool: &PgPool,
    role: UserRole,
    is_system_admin: bool,
    password: &str,
) -> User {
    let password_hash = hash_password(password).expect("hash password");
    insert_user_with_password_hash(pool, role, is_system_admin, password_hash).await
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
    .bind(date - ChronoDuration::days(7))
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

    fn restore_env(original: (Option<String>, Option<String>)) {
        match original.0 {
            Some(value) => env::set_var("TEST_DATABASE_URL", value),
            None => env::remove_var("TEST_DATABASE_URL"),
        }
        match original.1 {
            Some(value) => env::set_var("DATABASE_URL", value),
            None => env::remove_var("DATABASE_URL"),
        }
    }

    #[test]
    fn test_config_uses_database_url_from_env() {
        let _guard = env_guard();
        let original = (
            env::var("TEST_DATABASE_URL").ok(),
            env::var("DATABASE_URL").ok(),
        );
        env::set_var("TEST_DATABASE_URL", "postgres://override/testdb");

        let config = test_config();

        assert_eq!(config.database_url, "postgres://override/testdb");
        restore_env(original);
    }

    #[test]
    fn test_config_falls_back_to_default_when_env_missing() {
        let _guard = env_guard();
        let original = (
            env::var("TEST_DATABASE_URL").ok(),
            env::var("DATABASE_URL").ok(),
        );
        env::remove_var("TEST_DATABASE_URL");

        let config = test_config();
        let expected = env::var("DATABASE_URL").expect("database url set");

        assert_eq!(config.database_url, expected);
        restore_env(original);
    }
}
