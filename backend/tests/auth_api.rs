use axum::{extract::State, http::StatusCode, Json};
use chrono::Utc;
use sqlx::PgPool;
use timekeeper_backend::{
    handlers::auth::login,
    models::user::{LoginRequest, UserRole},
    utils::mfa::generate_totp_secret,
};
use totp_rs::{Algorithm, TOTP};
use base32::Alphabet::RFC4648;
mod support;
use support::{seed_user_with_password, test_config};

fn init_tracing() {
    let _ = tracing_subscriber::fmt::try_init();
}

#[sqlx::test(migrations = "./migrations")]
async fn login_returns_tokens_for_valid_credentials(pool: PgPool) {
    init_tracing();
    let config = test_config();
    let password = "CorrectHorseBatteryStaple1!";
    let user = seed_user_with_password(&pool, UserRole::Employee, false, password).await;

    let payload = LoginRequest {
        username: user.username.clone(),
        password: password.to_string(),
        totp_code: None,
    };

    let Json(response) = login(State((pool.clone(), config)), Json(payload))
        .await
        .expect("login succeeds");

    assert_eq!(response.user.username, user.username);
    assert!(!response.access_token.is_empty());
    assert!(!response.refresh_token.is_empty());
}

#[sqlx::test(migrations = "./migrations")]
async fn login_succeeds_for_default_admin(pool: PgPool) {
    init_tracing();
    let config = test_config();

    let payload = LoginRequest {
        username: "admin".into(),
        password: "admin123".into(),
        totp_code: None,
    };

    let Json(response) = login(State((pool.clone(), config)), Json(payload))
        .await
        .expect("default admin login succeeds");

    assert_eq!(response.user.username, "admin");
    assert_eq!(response.user.role, "admin");
    assert!(!response.access_token.is_empty());
}

#[sqlx::test(migrations = "./migrations")]
async fn login_with_mfa_requires_code_and_accepts_valid_totp(pool: PgPool) {
    init_tracing();
    let config = test_config();
    let password = "ValidPassword1!";
    let user = seed_user_with_password(&pool, UserRole::Employee, false, password).await;
    let secret = generate_totp_secret();

    sqlx::query("UPDATE users SET mfa_secret = $1, mfa_enabled_at = $2 WHERE id = $3")
        .bind(&secret)
        .bind(Utc::now())
        .bind(&user.id)
        .execute(&pool)
        .await
        .expect("enable MFA");

    let missing_code_payload = LoginRequest {
        username: user.username.clone(),
        password: password.to_string(),
        totp_code: None,
    };

    let error = login(State((pool.clone(), config.clone())), Json(missing_code_payload))
        .await
        .expect_err("MFA should require a code");
    let (status, Json(body)) = error;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body.get("error").and_then(|v| v.as_str()), Some("MFA code required"));

    let totp_code = current_totp_code(&secret);
    let spaced_code = format!("{} {}", &totp_code[..3], &totp_code[3..]);
    let payload = LoginRequest {
        username: user.username.clone(),
        password: password.to_string(),
        totp_code: Some(spaced_code),
    };

    let Json(response) = login(State((pool.clone(), config)), Json(payload))
        .await
        .expect("login with MFA succeeds even if the code contains a space");

    assert_eq!(response.user.username, user.username);
    assert!(!response.access_token.is_empty());
    assert!(!response.refresh_token.is_empty());
}

fn current_totp_code(secret: &str) -> String {
    let cleaned = secret.trim().replace(' ', "").to_uppercase();
    let secret_bytes = base32::decode(RFC4648 { padding: false }, cleaned.as_str())
        .expect("valid base32 secret");
    TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        None,
        "test".into(),
    )
    .expect("build totp")
    .generate_current()
    .expect("generate totp")
}
