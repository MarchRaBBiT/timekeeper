use std::{
    future::Future,
    sync::{Arc, Mutex},
};

use chrono::Utc;
use chrono_tz::UTC;
use timekeeper_backend::{
    config::Config,
    error::AppError,
    handlers::auth::process_login_for_user,
    models::user::{LoginRequest, User, UserRole},
    utils::{cookies::SameSite, mfa::generate_totp_secret, password::hash_password},
};

const TEST_PASSWORD: &str = "correct-horse-battery-staple";

fn test_config() -> Config {
    Config {
        database_url: "".into(),
        jwt_secret: "a-secure-test-secret-that-is-long-enough".repeat(2),
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
        time_zone: UTC,
        mfa_issuer: "Timekeeper Test".into(),
        rate_limit_ip_max_requests: 15,
        rate_limit_ip_window_seconds: 900,
        rate_limit_user_max_requests: 20,
        rate_limit_user_window_seconds: 3600,
    }
}

fn block_on_future<F, T>(future: F) -> T
where
    F: Future<Output = T>,
{
    tokio::runtime::Runtime::new()
        .expect("create runtime")
        .block_on(future)
}

fn user_with_mfa_secret(secret: String, enabled: bool) -> User {
    let mut user = User::new(
        "tester".into(),
        hash_password(TEST_PASSWORD).expect("hash test password"),
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
    let user = User {
        password_hash: password_hash.clone(),
        ..user_with_mfa_secret(String::new(), false)
    };
    let config = test_config();
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let payload = LoginRequest {
        username: user.username.clone(),
        password: "correct-horse-battery-staple".into(),
        device_label: Some("unit-test".into()),
        totp_code: None,
    };

    let response = block_on_future(process_login_for_user(
        user,
        payload,
        &config,
        {
            let recorded = Arc::clone(&recorded);
            move |token| {
                let recorded = Arc::clone(&recorded);
                async move {
                    recorded.lock().unwrap().push(token.encoded());
                    Ok(())
                }
            }
        },
        {
            let recorded = Arc::clone(&recorded);
            move |claims, _| {
                let recorded = Arc::clone(&recorded);
                async move {
                    recorded.lock().unwrap().push(claims.jti.clone());
                    Ok(())
                }
            }
        },
    ))
    .expect("login should succeed");

    assert!(!response.access_token.is_empty());
    assert!(!response.refresh_token.is_empty());
    assert_eq!(response.user.username, "tester");
    let recorded = recorded.lock().unwrap();
    assert_eq!(
        recorded.len(),
        2,
        "refresh token and access token jti should be recorded"
    );
    assert_ne!(recorded[0], recorded[1]);
    assert_eq!(recorded[0], response.refresh_token);
}

#[test]
fn login_rejects_invalid_password_without_db() {
    let password_hash = hash_password("expected-secret").expect("hash password");
    let user = User {
        password_hash: password_hash.clone(),
        ..user_with_mfa_secret(String::new(), false)
    };
    let config = test_config();
    let payload = LoginRequest {
        username: user.username.clone(),
        password: "wrong-secret".into(),
        device_label: None,
        totp_code: None,
    };

    let err = block_on_future(process_login_for_user(
        user,
        payload,
        &config,
        |_| async { Ok(()) },
        |_, _| async { Ok(()) },
    ))
    .expect_err("mismatched password should fail");
    match err {
        AppError::Unauthorized(msg) => assert_eq!(msg, "Invalid username or password"),
        e => panic!("unexpected error: {:?}", e),
    }
}

#[test]
fn login_rejects_invalid_totp_code_when_mfa_is_enabled() {
    let secret = generate_totp_secret();
    let user = user_with_mfa_secret(secret, true);
    let config = test_config();
    let payload = LoginRequest {
        username: user.username.clone(),
        password: TEST_PASSWORD.into(),
        device_label: None,
        totp_code: Some("000000".into()),
    };

    let err = block_on_future(process_login_for_user(
        user,
        payload,
        &config,
        |_| async { Ok(()) },
        |_, _| async { Ok(()) },
    ))
    .expect_err("invalid code should be rejected");
    match err {
        AppError::Unauthorized(msg) => assert_eq!(msg, "Invalid MFA code"),
        e => panic!("unexpected error: {:?}", e),
    }
}

#[test]
fn login_requires_totp_code_when_mfa_is_enabled() {
    let secret = generate_totp_secret();
    let user = user_with_mfa_secret(secret, true);
    let config = test_config();
    let payload = LoginRequest {
        username: user.username.clone(),
        password: TEST_PASSWORD.into(),
        device_label: None,
        totp_code: None,
    };

    let err = block_on_future(process_login_for_user(
        user,
        payload,
        &config,
        |_| async { Ok(()) },
        |_, _| async { Ok(()) },
    ))
    .expect_err("missing code should be rejected");
    match err {
        AppError::Unauthorized(msg) => assert_eq!(msg, "MFA code required"),
        e => panic!("unexpected error: {:?}", e),
    }
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

    let config = test_config();
    let payload = LoginRequest {
        username: user.username.clone(),
        password: TEST_PASSWORD.into(),
        device_label: Some("device".into()),
        totp_code: Some(code),
    };

    block_on_future(process_login_for_user(
        user,
        payload,
        &config,
        |_| async { Ok(()) },
        |_, context| async move {
            assert_eq!(context.as_deref(), Some("device"));
            Ok(())
        },
    ))
    .expect("valid code should pass");
}
