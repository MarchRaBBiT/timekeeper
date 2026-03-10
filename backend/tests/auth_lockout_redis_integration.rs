use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    routing::post,
    Extension, Router,
};
use bb8_redis::redis;
use sqlx::PgPool;
use std::{
    env, fs,
    net::TcpListener,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, OnceLock},
};
use testcontainers::{clients::Cli, core::WaitFor, GenericImage, RunnableImage};
use timekeeper_backend::{
    config::Config,
    db::redis::create_redis_pool,
    handlers::auth,
    middleware::request_id::RequestId,
    models::user::UserRole,
    services::{
        audit_log::{AuditLogService, AuditLogServiceTrait},
        lockout_notification_queue::{LockoutNotificationJob, LOCKOUT_NOTIFICATION_QUEUE_KEY},
    },
    state::AppState,
};
use tower::ServiceExt;

mod support;

static DOCKER_WRAPPER_DIR: OnceLock<PathBuf> = OnceLock::new();

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

async fn migrate_db(pool: &PgPool) {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .expect("run migrations");
}

async fn integration_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static GUARD: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();
    GUARD
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

fn test_config(redis_url: String) -> Config {
    let mut config = support::test_config();
    config.redis_url = Some(redis_url);
    config.feature_redis_cache_enabled = true;
    config.account_lockout_threshold = 3;
    config.account_lockout_duration_minutes = 15;
    config.account_lockout_backoff_enabled = true;
    config.account_lockout_max_duration_hours = 24;
    config
}

async fn auth_router_with_redis(pool: PgPool, config: Config) -> Router {
    let redis_pool = create_redis_pool(&config)
        .await
        .expect("create redis pool")
        .expect("redis pool available");
    let state = AppState::new(pool.clone(), None, Some(redis_pool), None, config);
    let audit_log_service: Arc<dyn AuditLogServiceTrait> = Arc::new(AuditLogService::new(pool));
    Router::new()
        .route("/api/auth/login", post(auth::login))
        .layer(Extension(RequestId("test-request-id".to_string())))
        .layer(Extension(audit_log_service))
        .with_state(state)
}

async fn fetch_lock_state(
    pool: &PgPool,
    user_id: &str,
) -> (i32, Option<chrono::DateTime<chrono::Utc>>, i32) {
    sqlx::query_as::<_, (i32, Option<chrono::DateTime<chrono::Utc>>, i32)>(
        "SELECT failed_login_attempts, locked_until, lockout_count FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("fetch lock state")
}

async fn fetch_lockout_count(pool: &PgPool, user_id: &str) -> i32 {
    sqlx::query_scalar::<_, i32>("SELECT lockout_count FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .expect("fetch lockout count")
}

async fn redis_key_exists(redis_url: &str, user_id: &str) -> bool {
    let client = redis::Client::open(redis_url).expect("open redis client");
    let mut conn = client
        .get_multiplexed_async_connection()
        .await
        .expect("connect redis");
    let exists: i32 = redis::cmd("EXISTS")
        .arg(format!("auth:login-failures:{user_id}"))
        .query_async(&mut conn)
        .await
        .expect("check redis key");
    exists != 0
}

async fn flush_redis(redis_url: &str) {
    let client = redis::Client::open(redis_url).expect("open redis client");
    let mut conn = client
        .get_multiplexed_async_connection()
        .await
        .expect("connect redis");
    let _: () = redis::cmd("FLUSHDB")
        .query_async(&mut conn)
        .await
        .expect("flush redis");
}

async fn queued_lockout_notifications(redis_url: &str) -> Vec<LockoutNotificationJob> {
    let client = redis::Client::open(redis_url).expect("open redis client");
    let mut conn = client
        .get_multiplexed_async_connection()
        .await
        .expect("connect redis");
    let entries: Vec<String> = redis::cmd("LRANGE")
        .arg(LOCKOUT_NOTIFICATION_QUEUE_KEY)
        .arg(0)
        .arg(-1)
        .query_async(&mut conn)
        .await
        .expect("read lockout notification queue");
    entries
        .into_iter()
        .map(|entry| serde_json::from_str(&entry).expect("deserialize lockout notification job"))
        .collect()
}

#[tokio::test]
async fn login_failures_stay_in_redis_until_threshold_is_reached() {
    let _guard = integration_guard().await;
    ensure_docker_cli();
    let docker = Cli::default();
    let host_port = allocate_ephemeral_port();
    let image = GenericImage::new("redis", "7-alpine")
        .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"));
    let image = RunnableImage::from(image).with_mapped_port((host_port, 6379));
    let _container = docker.run(image);

    let redis_url = format!("redis://127.0.0.1:{host_port}");
    flush_redis(&redis_url).await;

    let pool = support::test_pool().await;
    migrate_db(&pool).await;
    let user =
        support::seed_user_with_password(&pool, UserRole::Employee, false, "Correct123!").await;
    let app = auth_router_with_redis(pool.clone(), test_config(redis_url.clone())).await;

    for attempt in 1..=2 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "username": user.username.clone(),
                            "password": "Wrong123!",
                        })
                        .to_string(),
                    ))
                    .expect("build login request"),
            )
            .await
            .expect("login request");
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "attempt {attempt}"
        );

        let (failed_attempts, locked_until, lockout_count) =
            fetch_lock_state(&pool, &user.id.to_string()).await;
        assert_eq!(failed_attempts, 0, "attempt {attempt}");
        assert!(locked_until.is_none(), "attempt {attempt}");
        assert_eq!(lockout_count, 0, "attempt {attempt}");
        assert!(redis_key_exists(&redis_url, &user.id.to_string()).await);
    }

    let threshold_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "username": user.username.clone(),
                        "password": "Wrong123!",
                    })
                    .to_string(),
                ))
                .expect("build login request"),
        )
        .await
        .expect("login request");
    assert_eq!(threshold_response.status(), StatusCode::UNAUTHORIZED);

    let (failed_attempts, locked_until, lockout_count) =
        fetch_lock_state(&pool, &user.id.to_string()).await;
    assert_eq!(failed_attempts, 0);
    assert!(locked_until.is_some());
    assert_eq!(lockout_count, 1);
    assert!(!redis_key_exists(&redis_url, &user.id.to_string()).await);
}

#[tokio::test]
async fn successful_login_clears_redis_failure_counter() {
    let _guard = integration_guard().await;
    ensure_docker_cli();
    let docker = Cli::default();
    let host_port = allocate_ephemeral_port();
    let image = GenericImage::new("redis", "7-alpine")
        .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"));
    let image = RunnableImage::from(image).with_mapped_port((host_port, 6379));
    let _container = docker.run(image);

    let redis_url = format!("redis://127.0.0.1:{host_port}");
    flush_redis(&redis_url).await;

    let pool = support::test_pool().await;
    migrate_db(&pool).await;
    let password = "Correct123!";
    let user = support::seed_user_with_password(&pool, UserRole::Employee, false, password).await;
    let app = auth_router_with_redis(pool.clone(), test_config(redis_url.clone())).await;

    let failed_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "username": user.username.clone(),
                        "password": "Wrong123!",
                    })
                    .to_string(),
                ))
                .expect("build failed login request"),
        )
        .await
        .expect("failed login request");
    assert_eq!(failed_response.status(), StatusCode::UNAUTHORIZED);
    assert!(redis_key_exists(&redis_url, &user.id.to_string()).await);

    let success_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "username": user.username.clone(),
                        "password": password,
                    })
                    .to_string(),
                ))
                .expect("build success login request"),
        )
        .await
        .expect("success login request");
    assert_eq!(success_response.status(), StatusCode::OK);

    let (failed_attempts, locked_until, lockout_count) =
        fetch_lock_state(&pool, &user.id.to_string()).await;
    assert_eq!(failed_attempts, 0);
    assert!(locked_until.is_none());
    assert_eq!(lockout_count, 0);
    assert!(!redis_key_exists(&redis_url, &user.id.to_string()).await);
}

#[tokio::test]
async fn login_falls_back_to_database_when_redis_becomes_unavailable() {
    let _guard = integration_guard().await;
    ensure_docker_cli();
    let docker = Cli::default();
    let host_port = allocate_ephemeral_port();
    let image = GenericImage::new("redis", "7-alpine")
        .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"));
    let image = RunnableImage::from(image).with_mapped_port((host_port, 6379));
    let container = docker.run(image);

    let redis_url = format!("redis://127.0.0.1:{host_port}");
    flush_redis(&redis_url).await;

    let pool = support::test_pool().await;
    migrate_db(&pool).await;
    let user =
        support::seed_user_with_password(&pool, UserRole::Employee, false, "Correct123!").await;
    let app = auth_router_with_redis(pool.clone(), test_config(redis_url)).await;

    drop(container);
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;

    for _ in 0..3 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "username": user.username.clone(),
                            "password": "Wrong123!",
                        })
                        .to_string(),
                    ))
                    .expect("build login request"),
            )
            .await
            .expect("login request");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    let (failed_attempts, locked_until, lockout_count) =
        fetch_lock_state(&pool, &user.id.to_string()).await;
    assert_eq!(failed_attempts, 0);
    assert!(locked_until.is_some());
    assert_eq!(lockout_count, 1);
}

#[tokio::test]
async fn redis_lockout_uses_decayed_history_after_quiet_period() {
    let _guard = integration_guard().await;
    ensure_docker_cli();
    let docker = Cli::default();
    let host_port = allocate_ephemeral_port();
    let image = GenericImage::new("redis", "7-alpine")
        .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"));
    let image = RunnableImage::from(image).with_mapped_port((host_port, 6379));
    let _container = docker.run(image);

    let redis_url = format!("redis://127.0.0.1:{host_port}");
    flush_redis(&redis_url).await;

    let pool = support::test_pool().await;
    migrate_db(&pool).await;
    let user =
        support::seed_user_with_password(&pool, UserRole::Employee, false, "Correct123!").await;
    sqlx::query(
        "UPDATE users \
         SET lockout_count = 3, \
             failed_login_attempts = 0, \
             locked_until = NULL, \
             last_login_failure_at = NOW() - INTERVAL '49 hours', \
             updated_at = NOW() \
         WHERE id = $1",
    )
    .bind(user.id.to_string())
    .execute(&pool)
    .await
    .expect("seed decayed lockout history");
    let mut config = test_config(redis_url.clone());
    config.account_lockout_threshold = 1;
    let app = auth_router_with_redis(pool.clone(), config).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "username": user.username.clone(),
                        "password": "Wrong123!",
                    })
                    .to_string(),
                ))
                .expect("build login request"),
        )
        .await
        .expect("login request");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    assert_eq!(fetch_lockout_count(&pool, &user.id.to_string()).await, 2);
    assert!(!redis_key_exists(&redis_url, &user.id.to_string()).await);
}

#[tokio::test]
async fn lockout_notification_is_enqueued_in_redis() {
    let _guard = integration_guard().await;
    ensure_docker_cli();
    let docker = Cli::default();
    let host_port = allocate_ephemeral_port();
    let image = GenericImage::new("redis", "7-alpine")
        .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"));
    let image = RunnableImage::from(image).with_mapped_port((host_port, 6379));
    let _container = docker.run(image);

    let redis_url = format!("redis://127.0.0.1:{host_port}");
    flush_redis(&redis_url).await;

    let pool = support::test_pool().await;
    migrate_db(&pool).await;
    let user =
        support::seed_user_with_password(&pool, UserRole::Employee, false, "Correct123!").await;
    let mut config = test_config(redis_url.clone());
    config.account_lockout_threshold = 1;
    let app = auth_router_with_redis(pool.clone(), config).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "username": user.username.clone(),
                        "password": "Wrong123!",
                    })
                    .to_string(),
                ))
                .expect("build login request"),
        )
        .await
        .expect("login request");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let jobs = queued_lockout_notifications(&redis_url).await;
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].user_id, user.id);
    assert!(jobs[0].locked_until > jobs[0].enqueued_at);
}
