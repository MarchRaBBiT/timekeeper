use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use sqlx::PgPool;

use crate::{config::Config, handlers::auth_repo, models::user::User, utils::jwt::Claims};

pub async fn auth(
    State((pool, config)): State<(PgPool, Config)>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .map(|value| value.to_owned());
    let (claims, user) = authenticate_request(auth_header.as_deref(), &pool, &config).await?;
    request.extensions_mut().insert(claims.clone());
    request.extensions_mut().insert(user);

    Ok(next.run(request).await)
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
    State((pool, config)): State<(PgPool, Config)>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .map(|value| value.to_owned());
    let (claims, user) = authenticate_request(auth_header.as_deref(), &pool, &config).await?;
    if !(user.is_admin() || user.is_system_admin()) {
        return Err(StatusCode::FORBIDDEN);
    }

    request.extensions_mut().insert(claims.clone());
    request.extensions_mut().insert(user);
    Ok(next.run(request).await)
}

// Auth + require system admin flag for system-level routes
pub async fn auth_system_admin(
    State((pool, config)): State<(PgPool, Config)>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .map(|value| value.to_owned());
    let (claims, user) = authenticate_request(auth_header.as_deref(), &pool, &config).await?;
    if !user.is_system_admin() {
        return Err(StatusCode::FORBIDDEN);
    }

    request.extensions_mut().insert(claims.clone());
    request.extensions_mut().insert(user);
    Ok(next.run(request).await)
}

async fn get_user_by_id(pool: &PgPool, user_id: &str) -> Result<Option<User>, sqlx::Error> {
    let user = sqlx::query_as::<_, User>(
        "SELECT id, username, password_hash, full_name, LOWER(role) as role, is_system_admin, \
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
    pool: &PgPool,
    config: &Config,
) -> Result<(Claims, User), StatusCode> {
    let token = match auth_header.and_then(parse_bearer_token) {
        Some(token) => token,
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    let claims = verify_token(token, &config.jwt_secret).map_err(|_| StatusCode::UNAUTHORIZED)?;

    let is_active = auth_repo::access_token_exists(pool, &claims.jti)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !is_active {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let user = get_user_by_id(pool, &claims.sub)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    Ok((claims, user))
}
