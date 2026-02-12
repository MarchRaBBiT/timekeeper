use chrono::{Duration as ChronoDuration, NaiveDate, Utc};
use std::sync::OnceLock;
use timekeeper_backend::{
    models::{
        leave_request::LeaveType,
        user::{User, UserRole},
    },
    repositories::user as user_repo,
    utils::encryption::hash_email,
};
use tokio::sync::Mutex;
use uuid::Uuid;

#[path = "support/mod.rs"]
mod support;

async fn integration_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD.get_or_init(|| Mutex::new(())).lock().await
}

async fn reset_tables(pool: &sqlx::PgPool) {
    sqlx::query(
        "TRUNCATE archived_break_records, archived_attendance, archived_leave_requests, \
         archived_overtime_requests, archived_holiday_exceptions, archived_users, \
         break_records, attendance, leave_requests, overtime_requests, holiday_exceptions, \
         active_sessions, refresh_tokens, active_access_tokens, users RESTART IDENTITY CASCADE",
    )
    .execute(pool)
    .await
    .expect("truncate user repository tables");
}

#[tokio::test]
async fn user_repository_basic_crud_and_mfa_flags() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_tables(&pool).await;

    let created = user_repo::create_user(
        &pool,
        &User::new(
            "repo-user".to_string(),
            "hash".to_string(),
            "Repo User".to_string(),
            "repo-user@example.com".to_string(),
            UserRole::Employee,
            false,
        ),
        &hash_email("repo-user@example.com", &support::test_config()),
    )
    .await
    .expect("create user");
    assert_eq!(created.username, "repo-user");

    let created_admin = user_repo::create_user(
        &pool,
        &User::new(
            "repo-admin".to_string(),
            "hash-admin".to_string(),
            "Repo Admin".to_string(),
            "repo-admin@example.com".to_string(),
            UserRole::Admin,
            true,
        ),
        &hash_email("repo-admin@example.com", &support::test_config()),
    )
    .await
    .expect("create admin user");
    assert!(matches!(created_admin.role, UserRole::Admin));

    let listed = user_repo::list_users(&pool).await.expect("list users");
    assert_eq!(listed.len(), 2);
    assert!(listed.iter().any(|u| u.id == created.id));
    assert!(listed.iter().any(|u| u.id == created_admin.id));

    assert!(user_repo::username_exists(&pool, "repo-user")
        .await
        .expect("username exists"));
    assert!(!user_repo::username_exists(&pool, "missing-user")
        .await
        .expect("username missing"));

    let created_id = created.id.to_string();
    assert!(user_repo::user_exists(&pool, &created_id)
        .await
        .expect("user exists by id"));
    assert_eq!(
        user_repo::fetch_username(&pool, &created_id)
            .await
            .expect("fetch username")
            .as_deref(),
        Some("repo-user")
    );
    assert!(
        user_repo::fetch_username(&pool, &Uuid::new_v4().to_string())
            .await
            .expect("fetch missing username")
            .is_none()
    );

    let other = support::seed_user(&pool, UserRole::Admin, false).await;
    assert!(!user_repo::email_exists_for_other_user(
        &pool,
        &hash_email("repo-user@example.com", &support::test_config()),
        "repo-user@example.com",
        &created_id
    )
    .await
    .expect("email excluded for self"));
    assert!(user_repo::email_exists_for_other_user(
        &pool,
        &hash_email("repo-user@example.com", &support::test_config()),
        "repo-user@example.com",
        &other.id.to_string()
    )
    .await
    .expect("email exists for other user"));

    let updated_profile = user_repo::update_profile(
        &pool,
        &created_id,
        "Updated Name",
        "updated-profile@example.com",
        &hash_email("updated-profile@example.com", &support::test_config()),
    )
    .await
    .expect("update profile");
    assert_eq!(updated_profile.full_name, "Updated Name");
    assert_eq!(updated_profile.email, "updated-profile@example.com");

    let updated_user = user_repo::update_user(
        &pool,
        &created_id,
        "Admin Name",
        "updated-admin@example.com",
        &hash_email("updated-admin@example.com", &support::test_config()),
        UserRole::Admin,
        true,
    )
    .await
    .expect("update user");
    assert!(matches!(updated_user.role, UserRole::Admin));
    assert!(updated_user.is_system_admin);

    let now = Utc::now();
    assert!(
        user_repo::set_mfa_secret(&pool, &created_id, "BASE32SECRET", now)
            .await
            .expect("set mfa secret")
    );
    assert!(
        user_repo::enable_mfa(&pool, &created_id, now + ChronoDuration::minutes(1))
            .await
            .expect("enable mfa")
    );
    assert!(user_repo::disable_mfa(&pool, &created_id)
        .await
        .expect("disable mfa"));
    assert!(user_repo::reset_mfa(&pool, &created_id)
        .await
        .expect("reset mfa"));

    let missing_id = Uuid::new_v4().to_string();
    assert!(
        !user_repo::set_mfa_secret(&pool, &missing_id, "BASE32SECRET", now)
            .await
            .expect("set mfa secret missing user")
    );
    assert!(!user_repo::enable_mfa(&pool, &missing_id, now)
        .await
        .expect("enable mfa missing user"));
    assert!(!user_repo::disable_mfa(&pool, &missing_id)
        .await
        .expect("disable mfa missing user"));
    assert!(!user_repo::reset_mfa(&pool, &missing_id)
        .await
        .expect("reset mfa missing user"));

    user_repo::hard_delete_user(&pool, &created_id)
        .await
        .expect("hard delete user");
    assert!(!user_repo::user_exists(&pool, &created_id)
        .await
        .expect("user deleted"));
}

#[tokio::test]
async fn user_repository_soft_delete_archives_records_and_hard_delete_archived_removes_all() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_tables(&pool).await;

    let user = support::seed_user(&pool, UserRole::Employee, false).await;
    let archiver = support::seed_user(&pool, UserRole::Admin, true).await;

    let date = NaiveDate::from_ymd_opt(2026, 2, 10).expect("valid date");
    let clock_in = date.and_hms_opt(9, 0, 0).expect("clock in");
    let clock_out = date.and_hms_opt(18, 0, 0).expect("clock out");

    let attendance =
        support::seed_attendance(&pool, user.id, date, Some(clock_in), Some(clock_out)).await;
    support::seed_break_record(
        &pool,
        attendance.id,
        clock_in + ChronoDuration::hours(2),
        Some(clock_in + ChronoDuration::hours(3)),
    )
    .await;
    support::seed_leave_request(&pool, user.id, LeaveType::Personal, date, date).await;
    support::seed_overtime_request(&pool, user.id, date, 1.5).await;
    support::seed_holiday_exception(&pool, user.id, date, false, "coverage test").await;

    let refresh_token_id = format!("refresh-{}", Uuid::new_v4());
    let access_jti = format!("jti-{}", Uuid::new_v4());
    support::seed_active_session(&pool, user.id, &refresh_token_id, Some(&access_jti)).await;
    sqlx::query(
        "INSERT INTO active_access_tokens (jti, user_id, expires_at, context) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(&access_jti)
    .bind(user.id.to_string())
    .bind(Utc::now() + ChronoDuration::hours(1))
    .bind(Some("coverage".to_string()))
    .execute(&pool)
    .await
    .expect("insert active access token");

    let user_id = user.id.to_string();
    let archiver_id = archiver.id.to_string();
    user_repo::soft_delete_user(&pool, &user_id, &archiver_id)
        .await
        .expect("soft delete user");

    assert!(!user_repo::user_exists(&pool, &user_id)
        .await
        .expect("user deleted from users"));
    assert!(user_repo::archived_user_exists(&pool, &user_id)
        .await
        .expect("user exists in archive"));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM archived_attendance WHERE user_id = $1")
            .bind(&user_id)
            .fetch_one(&pool)
            .await
            .expect("count archived attendance"),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) \
             FROM archived_break_records \
             WHERE attendance_id IN (SELECT id FROM archived_attendance WHERE user_id = $1)"
        )
        .bind(&user_id)
        .fetch_one(&pool)
        .await
        .expect("count archived breaks"),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM archived_leave_requests WHERE user_id = $1"
        )
        .bind(&user_id)
        .fetch_one(&pool)
        .await
        .expect("count archived leave requests"),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM archived_overtime_requests WHERE user_id = $1"
        )
        .bind(&user_id)
        .fetch_one(&pool)
        .await
        .expect("count archived overtime requests"),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM archived_holiday_exceptions WHERE user_id = $1"
        )
        .bind(&user_id)
        .fetch_one(&pool)
        .await
        .expect("count archived holiday exceptions"),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM refresh_tokens WHERE user_id = $1")
            .bind(&user_id)
            .fetch_one(&pool)
            .await
            .expect("count refresh tokens"),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM active_access_tokens WHERE user_id = $1"
        )
        .bind(&user_id)
        .fetch_one(&pool)
        .await
        .expect("count active access tokens"),
        0
    );

    user_repo::hard_delete_archived_user(&pool, &user_id)
        .await
        .expect("hard delete archived user");
    assert!(!user_repo::archived_user_exists(&pool, &user_id)
        .await
        .expect("archived user removed"));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM archived_attendance WHERE user_id = $1")
            .bind(&user_id)
            .fetch_one(&pool)
            .await
            .expect("count archived attendance after purge"),
        0
    );
}

#[tokio::test]
async fn user_repository_restore_missing_user_is_noop() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_tables(&pool).await;

    let missing_user_id = Uuid::new_v4().to_string();
    user_repo::restore_user(&pool, &missing_user_id)
        .await
        .expect("restoring missing user should be a no-op");

    assert!(!user_repo::user_exists(&pool, &missing_user_id)
        .await
        .expect("missing user still absent"));
    assert!(!user_repo::archived_user_exists(&pool, &missing_user_id)
        .await
        .expect("missing archived user still absent"));
}

#[tokio::test]
async fn user_repository_soft_delete_restore_and_archive_cleanup() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_tables(&pool).await;

    let user = support::seed_user(&pool, UserRole::Employee, false).await;
    let archiver = support::seed_user(&pool, UserRole::Admin, true).await;
    let date = NaiveDate::from_ymd_opt(2026, 2, 4).expect("valid date");
    let clock_in = date.and_hms_opt(9, 0, 0).expect("clock in");
    let clock_out = date.and_hms_opt(18, 0, 0).expect("clock out");

    let attendance =
        support::seed_attendance(&pool, user.id, date, Some(clock_in), Some(clock_out)).await;
    support::seed_break_record(
        &pool,
        attendance.id,
        clock_in + ChronoDuration::hours(3),
        Some(clock_in + ChronoDuration::hours(4)),
    )
    .await;
    support::seed_leave_request(&pool, user.id, LeaveType::Annual, date, date).await;
    support::seed_overtime_request(&pool, user.id, date, 2.0).await;
    support::seed_holiday_exception(&pool, user.id, date, false, "manual override").await;

    let user_id = user.id.to_string();
    let archiver_id = archiver.id.to_string();
    user_repo::soft_delete_user(&pool, &user_id, &archiver_id)
        .await
        .expect("soft delete user");

    assert!(!user_repo::user_exists(&pool, &user_id)
        .await
        .expect("user removed"));
    assert!(user_repo::archived_user_exists(&pool, &user_id)
        .await
        .expect("archived user exists"));
    assert_eq!(
        user_repo::fetch_archived_username(&pool, &user_id)
            .await
            .expect("fetch archived username")
            .as_deref(),
        Some(user.username.as_str())
    );

    let archived_rows = user_repo::get_archived_users(&pool)
        .await
        .expect("get archived users");
    assert!(archived_rows.iter().any(|row| row.id == user_id));

    user_repo::restore_user(&pool, &user_id)
        .await
        .expect("restore archived user");
    assert!(user_repo::user_exists(&pool, &user_id)
        .await
        .expect("restored user exists"));
    assert!(!user_repo::archived_user_exists(&pool, &user_id)
        .await
        .expect("archived user removed after restore"));
    let restored_email = sqlx::query_scalar::<_, String>("SELECT email FROM users WHERE id = $1")
        .bind(&user_id)
        .fetch_one(&pool)
        .await
        .expect("fetch restored email");
    assert_eq!(restored_email, user.email);

    assert!(!user_repo::archived_user_exists(&pool, &user_id)
        .await
        .expect("archived user deleted"));
    assert!(user_repo::fetch_archived_username(&pool, &user_id)
        .await
        .expect("missing archived username after deletion")
        .is_none());
}
