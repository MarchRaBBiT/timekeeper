use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::{config::Config, models::user::User, utils::jwt::Claims};

pub async fn auth(
    State((pool, config)): State<(PgPool, Config)>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    let token = match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            header.strip_prefix("Bearer ").unwrap_or("")
        }
        _ => {
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    // Verify JWT token
    let claims = match verify_token(token, &config.jwt_secret) {
        Ok(claims) => claims,
        Err(_) => return Err(StatusCode::UNAUTHORIZED),
    };

    // Get user from database
    let user = match get_user_by_id(&pool, &claims.sub).await {
        Ok(Some(user)) => user,
        Ok(None) => return Err(StatusCode::UNAUTHORIZED),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    // Add user to request extensions
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
        .and_then(|header| header.to_str().ok());

    let token = match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            header.strip_prefix("Bearer ").unwrap_or("")
        }
        _ => return Err(StatusCode::UNAUTHORIZED),
    };

    let claims = match verify_token(token, &config.jwt_secret) {
        Ok(claims) => claims,
        Err(_) => return Err(StatusCode::UNAUTHORIZED),
    };

    let user = match get_user_by_id(&pool, &claims.sub).await {
        Ok(Some(user)) => user,
        Ok(None) => return Err(StatusCode::UNAUTHORIZED),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    if !user.is_admin() {
        return Err(StatusCode::FORBIDDEN);
    }

    request.extensions_mut().insert(user);
    Ok(next.run(request).await)
}

async fn get_user_by_id(pool: &PgPool, user_id: &str) -> Result<Option<User>, sqlx::Error> {
    let user = sqlx::query_as::<_, User>(
        "SELECT id, username, password_hash, full_name, LOWER(role) as role, created_at, updated_at FROM users WHERE id = $1"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(user)
}

// Helper function to extract user from request extensions
pub fn get_current_user(request: &Request) -> Option<&User> {
    request.extensions().get::<User>()
}
