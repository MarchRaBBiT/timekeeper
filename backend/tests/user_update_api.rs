use sqlx::PgPool;
use timekeeper_backend::models::user::{UpdateProfile, UpdateUser, User};
use uuid::Uuid;

mod support;

async fn migrate_db(pool: &PgPool) {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .expect("run migrations");
}

fn unique_email(base: &str) -> String {
    let (local, domain) = base.split_once('@').unwrap_or((base, "example.com"));
    format!("{}+{}@{}", local, Uuid::new_v4(), domain)
}

#[tokio::test]
async fn test_admin_update_user_email() {
    let pool = support::test_pool().await;
    migrate_db(&pool).await;

    let admin_email = unique_email("admin@test.com");
    let user_email = unique_email("user@test.com");
    let _admin = create_test_user(&pool, &admin_email, "admin", true).await;
    let user = create_test_user(&pool, &user_email, "user1", false).await;

    let new_email = unique_email("newemail@test.com");

    let update_payload = UpdateUser {
        full_name: None,
        email: Some(new_email.clone()),
        role: None,
        is_system_admin: None,
    };

    let updated = sqlx::query_as::<_, User>(
         "UPDATE users SET full_name = COALESCE($1, full_name), email = COALESCE($2, email), \
         role = COALESCE($3, role), is_system_admin = COALESCE($4, is_system_admin), updated_at = NOW() \
         WHERE id = $5 \
         RETURNING id, username, password_hash, full_name, email, LOWER(role) as role, is_system_admin, \
         mfa_secret, mfa_enabled_at, password_changed_at, created_at, updated_at",
    )
    .bind(update_payload.full_name)
    .bind(update_payload.email)
    .bind(update_payload.role.map(|r| r.as_str()))
    .bind(update_payload.is_system_admin)
    .bind(user.id)
    .fetch_one(&pool)
    .await
    .expect("update user");

    assert_eq!(updated.email, new_email);
    assert_eq!(updated.id, user.id);
}

#[tokio::test]
async fn test_user_update_own_profile() {
    let pool = support::test_pool().await;
    migrate_db(&pool).await;

    let original_email = unique_email("original@test.com");
    let user = create_test_user(&pool, &original_email, "testuser", false).await;

    let new_email = unique_email("updated@test.com");
    let new_full_name = "Updated Name";

    let update_payload = UpdateProfile {
        full_name: Some(new_full_name.to_string()),
        email: Some(new_email.clone()),
    };

    let updated = sqlx::query_as::<_, User>(
         "UPDATE users SET full_name = COALESCE($1, full_name), email = COALESCE($2, email), updated_at = NOW() \
         WHERE id = $3 \
         RETURNING id, username, password_hash, full_name, email, LOWER(role) as role, is_system_admin, \
         mfa_secret, mfa_enabled_at, password_changed_at, created_at, updated_at",
    )
    .bind(update_payload.full_name)
    .bind(update_payload.email)
    .bind(user.id)
    .fetch_one(&pool)
    .await
    .expect("update profile");

    assert_eq!(updated.email, new_email);
    assert_eq!(updated.full_name, new_full_name);
}

#[tokio::test]
async fn test_email_uniqueness_check() {
    let pool = support::test_pool().await;
    migrate_db(&pool).await;

    let user1_email = unique_email("user1@test.com");
    let user2_email = unique_email("user2@test.com");
    let _user1 = create_test_user(&pool, &user1_email, "user1", false).await;
    let user2 = create_test_user(&pool, &user2_email, "user2", false).await;

    let email_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM users WHERE email = $1 AND id != $2)",
    )
    .bind(user1_email)
    .bind(user2.id)
    .fetch_one(&pool)
    .await
    .expect("check email");

    assert!(email_exists);
}

async fn create_test_user(pool: &PgPool, email: &str, username: &str, is_admin: bool) -> User {
    let password_hash =
        timekeeper_backend::utils::password::hash_password("TestPass123!").expect("hash password");

    let user_id = Uuid::new_v4();
    let unique_username = format!("{}_{}", username, user_id);
    sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (id, username, password_hash, full_name, email, role, is_system_admin)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, username, password_hash, full_name, email, LOWER(role) as role, is_system_admin, 
        mfa_secret, mfa_enabled_at, password_changed_at, created_at, updated_at
        "#,
    )
    .bind(user_id)
    .bind(unique_username)
    .bind(password_hash)
    .bind("Test User")
    .bind(email)
    .bind(if is_admin { "admin" } else { "employee" })
    .bind(is_admin)
    .fetch_one(pool)
    .await
    .expect("create test user")
}
