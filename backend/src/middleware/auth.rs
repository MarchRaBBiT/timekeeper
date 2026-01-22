use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use sqlx::PgPool;

use crate::types::UserId;
use crate::{
    models::user::User,
    repositories::auth as auth_repo,
    state::AppState,
    utils::{
        cookies::{extract_cookie_value, ACCESS_COOKIE_NAME},
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
        "SELECT id, username, password_hash, full_name, email, LOWER(role) as role, is_system_admin, \
         mfa_secret, mfa_enabled_at, created_at, updated_at FROM users WHERE id = $1",
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

    let user = get_user_by_id(&state.write_pool, &claims.sub)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

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
