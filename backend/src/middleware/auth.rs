use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use sqlx::PgPool;
use tracing::Span;

use crate::types::UserId;
use crate::{
    models::user::User,
    repositories::{active_session, auth as auth_repo},
    state::AppState,
    utils::{
        cookies::{extract_cookie_value, ACCESS_COOKIE_NAME},
        encryption::decrypt_pii,
        jwt::Claims,
    },
};
use chrono::Utc;
use std::str::FromStr;

pub async fn auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let (auth_header, cookie_header) = extract_auth_headers(request.headers());
    let (claims, user) =
        authenticate_request(auth_header.as_deref(), cookie_header.as_deref(), &state).await?;
    Span::current().record("user_id", user.id.to_string());
    Span::current().record("username", &user.username);
    request.extensions_mut().insert(claims.clone());
    request.extensions_mut().insert(user.clone());

    let mut response = next.run(request).await;
    response.extensions_mut().insert(user);
    Ok(response)
}

fn verify_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let validation = Validation::default();
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &validation,
    )?;

    Ok(token_data.claims)
}

// Auth + require admin role for admin-only routes
pub async fn auth_admin(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let (auth_header, cookie_header) = extract_auth_headers(request.headers());
    let (claims, user) =
        authenticate_request(auth_header.as_deref(), cookie_header.as_deref(), &state).await?;
    Span::current().record("user_id", user.id.to_string());
    Span::current().record("username", &user.username);
    if !(user.is_admin() || user.is_system_admin()) {
        let mut response = StatusCode::FORBIDDEN.into_response();
        response.extensions_mut().insert(user);
        return Ok(response);
    }

    request.extensions_mut().insert(claims.clone());
    request.extensions_mut().insert(user.clone());
    let mut response = next.run(request).await;
    response.extensions_mut().insert(user);
    Ok(response)
}

// Auth + require system admin flag for system-level routes
pub async fn auth_system_admin(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let (auth_header, cookie_header) = extract_auth_headers(request.headers());
    let (claims, user) =
        authenticate_request(auth_header.as_deref(), cookie_header.as_deref(), &state).await?;
    Span::current().record("user_id", user.id.to_string());
    Span::current().record("username", &user.username);
    if !user.is_system_admin() {
        let mut response = StatusCode::FORBIDDEN.into_response();
        response.extensions_mut().insert(user);
        return Ok(response);
    }

    request.extensions_mut().insert(claims.clone());
    request.extensions_mut().insert(user.clone());
    let mut response = next.run(request).await;
    response.extensions_mut().insert(user);
    Ok(response)
}

async fn get_user_by_id(pool: &PgPool, user_id: &str) -> Result<Option<User>, sqlx::Error> {
    let user = sqlx::query_as::<_, User>(
        "SELECT id, username, password_hash, COALESCE(full_name_enc, '') as full_name, \
         COALESCE(email_enc, '') as email, LOWER(role) as role, is_system_admin, \
         mfa_secret_enc as mfa_secret, mfa_enabled_at, password_changed_at, failed_login_attempts, locked_until, lock_reason, lockout_count, created_at, updated_at \
         FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    Ok(user)
}
fn parse_bearer_token(header: &str) -> Option<&str> {
    if let Some(rest) = header.strip_prefix("Bearer ") {
        return Some(rest);
    }
    if let Some(rest) = header.strip_prefix("bearer ") {
        return Some(rest);
    }
    if let Some(space_idx) = header.find(' ') {
        let (scheme, rest) = header.split_at(space_idx);
        if scheme.eq_ignore_ascii_case("bearer") {
            return Some(rest.trim_start());
        }
    }
    None
}

async fn authenticate_request(
    auth_header: Option<&str>,
    cookie_header: Option<&str>,
    state: &AppState,
) -> Result<(Claims, User), StatusCode> {
    let token = auth_header
        .and_then(parse_bearer_token)
        .map(|value| value.to_string())
        .or_else(|| cookie_header.and_then(|raw| extract_cookie_value(raw, ACCESS_COOKIE_NAME)))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let claims =
        verify_token(&token, &state.config.jwt_secret).map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Cache-aside pattern for token validation
    let is_active = if state.config.feature_redis_cache_enabled {
        if let Some(cache) = &state.token_cache {
            match cache.is_token_active(&claims.jti).await {
                Ok(Some(active)) => active,
                _ => {
                    // Fallback to DB
                    let active = auth_repo::access_token_exists(&state.write_pool, &claims.jti)
                        .await
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                    // Try to backfill cache if token is active
                    if active {
                        if let Ok(user_id) = UserId::from_str(&claims.sub) {
                            let _ = cache
                                .cache_token(
                                    &claims.jti,
                                    user_id,
                                    (claims.exp - Utc::now().timestamp()).max(0) as u64,
                                )
                                .await;
                        } else {
                            tracing::warn!(
                                jti = %claims.jti,
                                sub = %claims.sub,
                                "Skipping cache backfill for invalid user id"
                            );
                        }
                    }
                    active
                }
            }
        } else {
            auth_repo::access_token_exists(&state.write_pool, &claims.jti)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        }
    } else {
        auth_repo::access_token_exists(&state.write_pool, &claims.jti)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };

    if !is_active {
        return Err(StatusCode::UNAUTHORIZED);
    }

    if let Err(err) = active_session::touch_active_session_by_access_jti(
        &state.write_pool,
        &claims.jti,
        Utc::now(),
    )
    .await
    {
        tracing::warn!(error = ?err, jti = %claims.jti, "Failed to update active session");
    }

    let mut user = get_user_by_id(&state.write_pool, &claims.sub)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    user.full_name =
        decrypt_pii(&user.full_name, &state.config).map_err(|_| StatusCode::UNAUTHORIZED)?;
    user.email = decrypt_pii(&user.email, &state.config).map_err(|_| StatusCode::UNAUTHORIZED)?;
    if let Some(secret) = user.mfa_secret.clone() {
        user.mfa_secret =
            Some(decrypt_pii(&secret, &state.config).map_err(|_| StatusCode::UNAUTHORIZED)?);
    }

    Ok((claims, user))
}

fn extract_auth_headers(headers: &axum::http::HeaderMap) -> (Option<String>, Option<String>) {
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_owned());
    let cookie_header = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_owned());
    (auth_header, cookie_header)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    #[test]
    fn test_parse_bearer_token_with_bearer_prefix() {
        let header = "Bearer test_token_123";
        let result = parse_bearer_token(header);
        assert_eq!(result, Some("test_token_123"));
    }

    #[test]
    fn test_parse_bearer_token_lowercase_bearer() {
        let header = "bearer test_token_123";
        let result = parse_bearer_token(header);
        assert_eq!(result, Some("test_token_123"));
    }

    #[test]
    fn test_parse_bearer_token_with_multiple_spaces() {
        let header = "Bearer   test_token_123";
        let result = parse_bearer_token(header);
        assert_eq!(result, Some("  test_token_123"));
    }

    #[test]
    fn test_parse_bearer_token_returns_none_for_invalid_prefix() {
        let header = "Basic test_token_123";
        let result = parse_bearer_token(header);
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_bearer_token_returns_none_for_missing_token() {
        let header = "Bearer ";
        let result = parse_bearer_token(header);
        assert_eq!(result, Some(""));
    }

    #[test]
    fn test_parse_bearer_token_returns_none_for_empty_string() {
        let header = "";
        let result = parse_bearer_token(header);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_auth_headers_with_authorization() {
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, "Bearer test_token".parse().unwrap());
        let (auth, cookie) = extract_auth_headers(&headers);
        assert_eq!(auth, Some("Bearer test_token".to_string()));
        assert_eq!(cookie, None);
    }

    #[test]
    fn test_extract_auth_headers_with_cookie() {
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, "access_token=cookie_token".parse().unwrap());
        let (auth, cookie) = extract_auth_headers(&headers);
        assert_eq!(auth, None);
        assert_eq!(cookie, Some("access_token=cookie_token".to_string()));
    }

    #[test]
    fn test_extract_auth_headers_with_both() {
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, "Bearer test_token".parse().unwrap());
        headers.insert(header::COOKIE, "access_token=cookie_token".parse().unwrap());
        let (auth, cookie) = extract_auth_headers(&headers);
        assert_eq!(auth, Some("Bearer test_token".to_string()));
        assert_eq!(cookie, Some("access_token=cookie_token".to_string()));
    }

    #[test]
    fn test_extract_auth_headers_empty() {
        let headers = HeaderMap::new();
        let (auth, cookie) = extract_auth_headers(&headers);
        assert_eq!(auth, None);
        assert_eq!(cookie, None);
    }

    #[test]
    fn test_verify_token_with_valid_token() {
        use crate::models::user::UserRole;
        use crate::types::UserId;
        use crate::utils::jwt::{create_access_token, verify_access_token};

        let user_id = UserId::new();
        let username = "testuser".to_string();
        let role = format!("{:?}", UserRole::Employee);
        let secret = "test_secret_key_for_jwt_tokens";
        let (token_string, _claims) =
            create_access_token(user_id.to_string(), username, role, secret, 1).unwrap();

        let result = verify_access_token(&token_string, secret);
        assert!(result.is_ok());

        let claims = result.unwrap();
        assert_eq!(claims.sub, user_id.to_string());
    }

    #[test]
    fn test_verify_token_with_invalid_secret() {
        use crate::models::user::UserRole;
        use crate::types::UserId;
        use crate::utils::jwt::{create_access_token, verify_access_token};

        let user_id = UserId::new();
        let username = "testuser".to_string();
        let role = format!("{:?}", UserRole::Employee);
        let secret = "correct_secret";
        let wrong_secret = "wrong_secret";
        let (token_string, _claims) =
            create_access_token(user_id.to_string(), username, role, secret, 1).unwrap();

        let result = verify_access_token(&token_string, wrong_secret);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_token_with_malformed_token() {
        let secret = "test_secret";
        let result = verify_token("not.a.valid.token", secret);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_token_with_empty_token() {
        let secret = "test_secret";
        let result = verify_token("", secret);
        assert!(result.is_err());
    }
}
