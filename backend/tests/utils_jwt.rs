use timekeeper_backend::utils::jwt::{
    create_access_token, create_refresh_token, decode_refresh_token, verify_access_token,
    verify_refresh_token, Claims,
};

#[test]
fn jwt_create_and_verify_access_token() {
    let (token, claims) = create_access_token(
        "user-123".into(),
        "testuser".into(),
        "employee".into(),
        "testsecret",
        1,
    )
    .expect("create token");

    assert!(!token.is_empty());
    assert_eq!(claims.sub, "user-123");
    assert_eq!(claims.username, "testuser");
    assert_eq!(claims.role, "employee");
}

#[test]
fn jwt_verify_with_wrong_secret_fails() {
    let (token, _) = create_access_token(
        "user-123".into(),
        "testuser".into(),
        "employee".into(),
        "secret1",
        1,
    )
    .expect("create token");

    let result = verify_access_token(&token, "secret2");
    assert!(result.is_err());
}

#[test]
fn jwt_expired_token_fails_verification() {
    let expired_claims = Claims {
        sub: "user-123".into(),
        username: "testuser".into(),
        role: "employee".into(),
        exp: chrono::Utc::now().timestamp() - 3600,
        iat: chrono::Utc::now().timestamp() - 7200,
        jti: uuid::Uuid::new_v4().to_string(),
    };

    let token = jsonwebtoken::encode(
        &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256),
        &expired_claims,
        &jsonwebtoken::EncodingKey::from_secret("secret".as_ref()),
    )
    .expect("encode token");

    let result = verify_access_token(&token, "secret");
    assert!(result.is_err());
}

#[test]
fn jwt_malformed_token_fails() {
    let result = verify_access_token("invalid.token.here", "secret");
    assert!(result.is_err());
}

#[test]
fn jwt_create_and_verify_refresh_token() {
    let refresh_token = create_refresh_token("user-456".into(), 7).expect("create refresh token");

    assert!(!refresh_token.id.is_empty());
    assert!(!refresh_token.secret.is_empty());
    assert!(!refresh_token.token_hash.is_empty());
    assert_eq!(refresh_token.user_id, "user-456");
}

#[test]
fn jwt_verify_refresh_token_with_correct_hash() {
    let refresh_token = create_refresh_token("user-789".into(), 1).expect("create token");
    let is_valid = verify_refresh_token(&refresh_token.secret, &refresh_token.token_hash)
        .expect("verify refresh token");
    assert!(is_valid);
}

#[test]
fn jwt_verify_refresh_token_with_wrong_secret_fails() {
    let refresh_token = create_refresh_token("user-789".into(), 1).expect("create token");
    let is_valid = verify_refresh_token("wrong-secret", &refresh_token.token_hash)
        .expect("verify refresh token");
    assert!(!is_valid);
}

#[test]
fn jwt_decode_refresh_token_succeeds() {
    let refresh_token = create_refresh_token("user-abc".into(), 1).expect("create token");
    let encoded = refresh_token.encoded();

    let (id, secret) = decode_refresh_token(&encoded).expect("decode token");
    assert_eq!(id, refresh_token.id);
    assert_eq!(secret, refresh_token.secret);
}

#[test]
fn jwt_decode_malformed_refresh_token_fails() {
    let result = decode_refresh_token("no-colon-here");
    assert!(result.is_err());
}

#[test]
fn jwt_decode_empty_id_fails() {
    let result = decode_refresh_token(":secret");
    assert!(result.is_err());
}

#[test]
fn jwt_decode_empty_secret_fails() {
    let result = decode_refresh_token("id:");
    assert!(result.is_err());
}

#[test]
fn jwt_claims_has_unique_jti() {
    let (_, claims1) =
        create_access_token("user".into(), "name".into(), "role".into(), "secret", 1).unwrap();
    let (_, claims2) =
        create_access_token("user".into(), "name".into(), "role".into(), "secret", 1).unwrap();

    assert_ne!(claims1.jti, claims2.jti);
}

#[test]
fn jwt_claims_expiration_set_correctly() {
    let expiration_hours = 2u64;
    let (_, claims) = create_access_token(
        "user".into(),
        "name".into(),
        "role".into(),
        "secret",
        expiration_hours,
    )
    .unwrap();

    let expected_exp = claims.iat + (expiration_hours as i64 * 3600);
    assert!((claims.exp - expected_exp).abs() <= 1);
}

#[test]
fn jwt_refresh_token_has_future_expiration() {
    let refresh_token = create_refresh_token("user".into(), 7).expect("create token");
    assert!(refresh_token.expires_at > chrono::Utc::now());
}

#[test]
fn jwt_refresh_token_hash_is_different_from_secret() {
    let refresh_token = create_refresh_token("user".into(), 7).expect("create token");
    assert_ne!(refresh_token.token_hash, refresh_token.secret);
}
