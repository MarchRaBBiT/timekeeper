#![allow(dead_code)]
use chrono::{Datelike, Duration as ChronoDuration, NaiveDate};
use chrono_tz::Asia::Tokyo;
use ctor::{ctor, dtor};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{
    env,
    fs,
    path::Path,
    path::PathBuf,
    process::Command,
    net::TcpListener,
    sync::{Mutex, OnceLock},
    time::Duration as StdDuration,
};
use testcontainers::{clients::Cli, core::WaitFor, Container, GenericImage, RunnableImage};
use timekeeper_backend::{
    config::Config,
    models::user::{User, UserRole},
    models::{
        holiday::Holiday,
        leave_request::{LeaveRequest, LeaveType},
        overtime_request::OvertimeRequest,
    },
    types::{HolidayExceptionId, UserId, WeeklyHolidayId},
    utils::{cookies::SameSite, password::hash_password},
};
use uuid::Uuid;

static TESTCONTAINERS_DOCKER: OnceLock<&'static Cli> = OnceLock::new();
static TESTCONTAINERS_PG: OnceLock<Mutex<Option<Container<'static, GenericImage>>>> = OnceLock::new();
static TESTCONTAINERS_DB_URL: OnceLock<String> = OnceLock::new();
static DOCKER_WRAPPER_DIR: OnceLock<PathBuf> = OnceLock::new();
static ENV_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

#[ctor]
fn init_test_database_url() {
    if env::var("TEST_DATABASE_URL").is_ok() {
        return;
    }

    let url = start_testcontainer_postgres();
    env::set_var("TEST_DATABASE_URL", url);
}

fn env_guard() -> std::sync::MutexGuard<'static, ()> {
    ENV_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("lock env")
}

fn start_testcontainer_postgres() -> String {
    let url = TESTCONTAINERS_DB_URL.get().cloned().unwrap_or_else(|| {
        ensure_docker_cli();
        let docker = TESTCONTAINERS_DOCKER
            .get_or_init(|| Box::leak(Box::new(Cli::default())));
        let image_ref = env::var("TESTCONTAINERS_POSTGRES_IMAGE")
            .unwrap_or_else(|_| "postgres:15-alpine".to_string());
        let (image_name, image_tag) = image_ref
            .split_once(':')
            .unwrap_or((image_ref.as_str(), "latest"));
        let host_port = allocate_ephemeral_port();
        let image = GenericImage::new(image_name, image_tag)
            .with_env_var("POSTGRES_USER", "timekeeper_test")
            .with_env_var("POSTGRES_PASSWORD", "timekeeper_test")
            .with_env_var("POSTGRES_DB", "postgres")
            .with_wait_for(WaitFor::message_on_stdout(
                "database system is ready to accept connections",
            ));
        let image = RunnableImage::from(image).with_mapped_port((host_port, 5432));
        let container = docker.run(image);
        let holder = TESTCONTAINERS_PG.get_or_init(|| Mutex::new(None));
        let mut guard = holder.lock().expect("lock testcontainers postgres");
        *guard = Some(container);
        let url = format!(
            "postgres://timekeeper_test:timekeeper_test@127.0.0.1:{}/postgres",
            host_port
        );
        eprintln!("--- Testcontainers Postgres started at {} ---", url);
        TESTCONTAINERS_DB_URL
            .set(url.clone())
            .expect("set test database url");
        url
    });
    env::set_var("DATABASE_URL", url.clone());
    env::set_var("TEST_DATABASE_URL", url.clone());
    url
}

#[dtor]
fn shutdown_testcontainer_postgres() {
    if let Some(holder) = TESTCONTAINERS_PG.get() {
        if let Ok(mut guard) = holder.lock() {
            let _ = guard.take();
        }
    }
}

fn allocate_ephemeral_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind ephemeral port")
        .local_addr()
        .expect("read socket addr")
        .port()
}

fn ensure_docker_cli() {
    if env::var("DOCKER_HOST").is_err() {
        let podman_socket = Path::new("/run/podman/podman.sock");
        if podman_socket.exists() {
            env::set_var("DOCKER_HOST", "unix:///run/podman/podman.sock");
        } else if let Ok(runtime_dir) = env::var("XDG_RUNTIME_DIR") {
            let path = Path::new(&runtime_dir).join("podman/podman.sock");
            if path.exists() {
                if let Some(path_str) = path.to_str() {
                    env::set_var("DOCKER_HOST", format!("unix://{}", path_str));
                }
            }
        }
    }
    if Command::new("docker").arg("--version").output().is_ok() {
        return;
    }
    if Command::new("podman").arg("--version").output().is_err() {
        return;
    }
    let dir = DOCKER_WRAPPER_DIR.get_or_init(|| {
        let dir = env::temp_dir().join("timekeeper-testcontainers-docker");
        let _ = fs::create_dir_all(&dir);
        dir
    });
    let docker_path = dir.join("docker");
    if !docker_path.exists() {
        let script = "#!/usr/bin/env sh\nexec podman \"$@\"\n";
        let _ = fs::write(&docker_path, script);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = fs::metadata(&docker_path) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o755);
                let _ = fs::set_permissions(&docker_path, perms);
            }
        }
    }
    let path = env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", dir.display(), path);
    env::set_var("PATH", new_path);
}

pub fn test_config() -> Config {
    let database_url = test_database_url();

    Config {
        database_url,
        read_database_url: None,
        jwt_secret: "a_secure_token_that_is_long_enough_123".into(),
        jwt_expiration_hours: 1,
        refresh_token_expiration_days: 7,
        audit_log_retention_days: 1825,
        audit_log_retention_forever: false,
        consent_log_retention_days: 1825,
        consent_log_retention_forever: false,
        aws_region: "ap-northeast-1".into(),
        aws_kms_key_id: "alias/timekeeper-test".into(),
        aws_audit_log_bucket: "timekeeper-audit-logs".into(),
        aws_cloudtrail_enabled: true,
        cookie_secure: false,
        cookie_same_site: SameSite::Lax,
        cors_allow_origins: vec!["http://localhost:8000".into()],
        time_zone: Tokyo,
        mfa_issuer: "Timekeeper".into(),
        rate_limit_ip_max_requests: 15,
        rate_limit_ip_window_seconds: 900,
        rate_limit_user_max_requests: 20,
        rate_limit_user_window_seconds: 3600,
        redis_url: None,
        redis_pool_size: 10,
        redis_connect_timeout: 5,
        feature_redis_cache_enabled: true,
        feature_read_replica_enabled: true,
        password_min_length: 12,
        password_require_uppercase: true,
        password_require_lowercase: true,
        password_require_numbers: true,
        password_require_symbols: true,
        password_expiration_days: 90,
        password_history_count: 5,
        production_mode: false,
    }
}

pub async fn test_pool() -> PgPool {
    let database_url = test_database_url();
    let mut retry_count = 0;
    let max_retries = 3;

    loop {
        match PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(StdDuration::from_secs(30))
            .connect(&database_url)
            .await
        {
            Ok(pool) => return pool,
            Err(e) if retry_count < max_retries => {
                retry_count += 1;
                eprintln!(
                    "Retrying DB connection (attempt {}/{}): {}",
                    retry_count, max_retries, e
                );
                tokio::time::sleep(StdDuration::from_secs(2)).await;
            }
            Err(e) => panic!(
                "Failed to connect to test database after {} retries: {}",
                max_retries, e
            ),
        }
    }
}

fn test_database_url() -> String {
    let _guard = ENV_MUTEX.get_or_init(|| Mutex::new(())).try_lock().ok();
    env::var("TEST_DATABASE_URL")
        .or_else(|_| env::var("DATABASE_URL"))
        .unwrap_or_else(|_| start_testcontainer_postgres())
}

async fn insert_user_with_password_hash(
    pool: &PgPool,
    role: UserRole,
    is_system_admin: bool,
    password_hash: String,
) -> User {
    let user = User::new(
        format!("user_{}", Uuid::new_v4()),
        password_hash,
        "Test User".into(),
        format!("user_{}@example.com", Uuid::new_v4()),
        role,
        is_system_admin,
    );
    sqlx::query(
        "INSERT INTO users (id, username, password_hash, full_name, email, role, is_system_admin, \
         mfa_secret, mfa_enabled_at, password_changed_at, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
    )
    .bind(user.id.to_string())
    .bind(&user.username)
    .bind(&user.password_hash)
    .bind(&user.full_name)
    .bind(&user.email)
    .bind(user.role.as_str())
    .bind(user.is_system_admin)
    .bind(&user.mfa_secret)
    .bind(user.mfa_enabled_at)
    .bind(user.password_changed_at)
    .bind(user.created_at)
    .bind(user.updated_at)
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

pub async fn grant_permission(pool: &PgPool, user_id: &str, permission: &str) {
    sqlx::query(
        "INSERT INTO user_permissions (user_id, permission_name) VALUES ($1, $2) \
         ON CONFLICT (user_id, permission_name) DO NOTHING",
    )
    .bind(user_id)
    .bind(permission)
    .execute(pool)
    .await
    .expect("grant permission");
}

pub async fn seed_weekly_holiday(pool: &PgPool, date: NaiveDate) {
    let weekday = date.weekday().num_days_from_monday() as i16;
    sqlx::query(
        "INSERT INTO weekly_holidays \
            (id, weekday, starts_on, ends_on, enforced_from, enforced_to, created_by, created_at, updated_at) \
         VALUES ($1, $2, $3, NULL, $4, NULL, $5, NOW(), NOW())",
    )
    .bind(WeeklyHolidayId::new().to_string())
    .bind(weekday)
    .bind(date - ChronoDuration::days(7))
    .bind(date)
    .bind(UserId::new().to_string())
    .execute(pool)
    .await
    .expect("insert weekly holiday");
}

pub async fn seed_leave_request(
    pool: &PgPool,
    user_id: UserId,
    leave_type: LeaveType,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> LeaveRequest {
    let request = LeaveRequest::new(
        user_id,
        leave_type,
        start_date,
        end_date,
        Some("test".into()),
    );
    sqlx::query(
        "INSERT INTO leave_requests (id, user_id, leave_type, start_date, end_date, reason, status, approved_by, approved_at, decision_comment, rejected_by, rejected_at, cancelled_at, created_at, updated_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)",
    )
    .bind(request.id.to_string())
    .bind(request.user_id.to_string())
    .bind(match request.leave_type {
        LeaveType::Annual => "annual",
        LeaveType::Sick => "sick",
        LeaveType::Personal => "personal",
        LeaveType::Other => "other",
    })
    .bind(request.start_date)
    .bind(request.end_date)
    .bind(&request.reason)
    .bind(match request.status {
        timekeeper_backend::models::leave_request::RequestStatus::Pending => "pending",
        timekeeper_backend::models::leave_request::RequestStatus::Approved => "approved",
        timekeeper_backend::models::leave_request::RequestStatus::Rejected => "rejected",
        timekeeper_backend::models::leave_request::RequestStatus::Cancelled => "cancelled",
    })
    .bind(request.approved_by.map(|id| id.to_string()))
    .bind(request.approved_at)
    .bind(&request.decision_comment)
    .bind(request.rejected_by.map(|id| id.to_string()))
    .bind(request.rejected_at)
    .bind(request.cancelled_at)
    .bind(request.created_at)
    .bind(request.updated_at)
    .execute(pool)
    .await
    .expect("insert leave request");
    request
}

pub async fn seed_overtime_request(
    pool: &PgPool,
    user_id: UserId,
    date: NaiveDate,
    planned_hours: f64,
) -> OvertimeRequest {
    let request = OvertimeRequest::new(user_id, date, planned_hours, Some("test OT".into()));
    sqlx::query(
        "INSERT INTO overtime_requests (id, user_id, date, planned_hours, reason, status, approved_by, approved_at, decision_comment, rejected_by, rejected_at, cancelled_at, created_at, updated_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
    )
    .bind(request.id.to_string())
    .bind(request.user_id.to_string())
    .bind(request.date)
    .bind(request.planned_hours)
    .bind(&request.reason)
    .bind(match request.status {
        timekeeper_backend::models::overtime_request::RequestStatus::Pending => "pending",
        timekeeper_backend::models::overtime_request::RequestStatus::Approved => "approved",
        timekeeper_backend::models::overtime_request::RequestStatus::Rejected => "rejected",
        timekeeper_backend::models::overtime_request::RequestStatus::Cancelled => "cancelled",
    })
    .bind(request.approved_by.map(|id| id.to_string()))
    .bind(request.approved_at)
    .bind(&request.decision_comment)
    .bind(request.rejected_by.map(|id| id.to_string()))
    .bind(request.rejected_at)
    .bind(request.cancelled_at)
    .bind(request.created_at)
    .bind(request.updated_at)
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
    .bind(holiday.id.to_string())
    .bind(holiday.holiday_date)
    .bind(&holiday.name)
    .bind(&holiday.description)
    .bind(holiday.created_at)
    .bind(holiday.updated_at)
    .execute(pool)
    .await
    .expect("insert holiday");
    holiday
}

pub async fn seed_holiday_exception(
    pool: &PgPool,
    user_id: UserId,
    date: NaiveDate,
    override_value: bool,
    reason: &str,
) {
    sqlx::query(
        "INSERT INTO holiday_exceptions \
            (id, user_id, exception_date, override, reason, created_by, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())",
    )
    .bind(HolidayExceptionId::new().to_string())
    .bind(user_id.to_string())
    .bind(date)
    .bind(override_value)
    .bind(reason)
    .bind(UserId::new().to_string())
    .execute(pool)
    .await
    .expect("insert holiday exception");
}

#[cfg(test)]
mod tests {
    use super::*;

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
        if env::var("TEST_DATABASE_URL").is_ok() {
            return;
        }
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
        if env::var("TEST_DATABASE_URL").is_ok() {
            return;
        }
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
