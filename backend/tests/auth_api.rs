use axum::http::StatusCode;
use chrono::Utc;
use timekeeper_backend::{
    handlers::auth::{enforce_mfa, ensure_password_matches},
    models::user::{User, UserRole},
    utils::{mfa::generate_totp_secret, password::hash_password},
};

fn user_with_mfa_secret(secret: String, enabled: bool) -> User {
    let mut user = User::new(
        "tester".into(),
        "hash".into(),
        "Tester".into(),
        UserRole::Employee,
        false,
    );
    user.mfa_secret = Some(secret);
    user.mfa_enabled_at = enabled.then_some(Utc::now());
    user
}

#[test]
fn login_succeeds_when_password_matches_without_db() {
    let password_hash = hash_password("correct-horse-battery-staple").expect("hash password");
    ensure_password_matches(
        "correct-horse-battery-staple",
        &password_hash,
        "Invalid username or password",
    )
    .expect("passwords should match");
}

#[test]
fn login_rejects_invalid_password_without_db() {
    let password_hash = hash_password("expected-secret").expect("hash password");
    let err = ensure_password_matches(
        "wrong-secret",
        &password_hash,
        "Invalid username or password",
    )
    .expect_err("mismatched password should fail");
    assert_eq!(err.0, StatusCode::UNAUTHORIZED);
    assert_eq!(
        err.1 .0.get("error").and_then(|v| v.as_str()),
        Some("Invalid username or password")
    );
}

#[test]
fn login_requires_totp_code_when_mfa_is_enabled() {
    let secret = generate_totp_secret();
    let user = user_with_mfa_secret(secret, true);
    let err = enforce_mfa(&user, None).expect_err("missing code should be rejected");
    assert_eq!(err.0, StatusCode::UNAUTHORIZED);
    assert_eq!(
        err.1 .0.get("error").and_then(|v| v.as_str()),
        Some("MFA code required")
    );
}

#[test]
fn login_accepts_valid_totp_code_when_mfa_is_enabled() {
    let secret = generate_totp_secret();
    let user = user_with_mfa_secret(secret.clone(), true);
    let secret_bytes = base32::decode(base32::Alphabet::RFC4648 { padding: false }, &secret)
        .expect("decode secret");
    let totp = totp_rs::TOTP::new(
        totp_rs::Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        None,
        "".into(),
    )
    .expect("build totp");
    let code = totp.generate_current().expect("generate code");

    enforce_mfa(&user, Some(&code)).expect("valid code should pass");
}
