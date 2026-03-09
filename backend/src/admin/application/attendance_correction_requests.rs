use serde::Deserialize;
use serde_json::json;

use crate::{
    error::AppError,
    models::{
        attendance_correction_request::{AttendanceCorrectionResponse, DecisionPayload},
        user::User,
    },
    repositories::attendance_correction_request::AttendanceCorrectionRequestRepository,
    types::UserId,
};

#[derive(Debug, Clone, Deserialize)]
pub struct AdminAttendanceCorrectionListQuery {
    pub status: Option<String>,
    pub user_id: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

pub async fn list_attendance_correction_requests(
    read_pool: &sqlx::PgPool,
    user: &User,
    query: AdminAttendanceCorrectionListQuery,
) -> Result<Vec<AttendanceCorrectionResponse>, AppError> {
    ensure_admin(user)?;

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
    let user_filter = parse_optional_user_id(query.user_id)?;

    let repo = AttendanceCorrectionRequestRepository::new();
    let list = repo
        .list_paginated(
            read_pool,
            query.status.as_deref(),
            user_filter,
            page,
            per_page,
        )
        .await?;

    list.into_iter()
        .map(|item| item.to_response().map_err(AppError::InternalServerError))
        .collect()
}

pub async fn get_attendance_correction_request_detail(
    read_pool: &sqlx::PgPool,
    user: &User,
    id: &str,
) -> Result<AttendanceCorrectionResponse, AppError> {
    ensure_admin(user)?;

    let repo = AttendanceCorrectionRequestRepository::new();
    let request = repo.find_by_id(read_pool, id).await?;
    request.to_response().map_err(AppError::InternalServerError)
}

pub async fn approve_attendance_correction_request(
    write_pool: &sqlx::PgPool,
    user: &User,
    id: &str,
    payload: DecisionPayload,
) -> Result<serde_json::Value, AppError> {
    ensure_admin(user)?;
    validate_comment(&payload.comment)?;

    let repo = AttendanceCorrectionRequestRepository::new();
    let request = repo.find_by_id(write_pool, id).await?;
    ensure_not_self_request(request.user_id, user.id)?;

    let original_snapshot = request
        .parse_original_snapshot()
        .map_err(|error| AppError::InternalServerError(error.into()))?;
    let proposed = request
        .parse_proposed_values()
        .map_err(|error| AppError::InternalServerError(error.into()))?;

    repo.approve_and_apply_effective_values(
        write_pool,
        id,
        request.attendance_id,
        user.id,
        &payload.comment,
        &original_snapshot,
        &proposed,
    )
    .await?;

    Ok(json!({ "message": "Request approved" }))
}

pub async fn reject_attendance_correction_request(
    write_pool: &sqlx::PgPool,
    user: &User,
    id: &str,
    payload: DecisionPayload,
) -> Result<serde_json::Value, AppError> {
    ensure_admin(user)?;
    validate_comment(&payload.comment)?;

    let repo = AttendanceCorrectionRequestRepository::new();
    let request = repo.find_by_id(write_pool, id).await?;
    ensure_not_self_request(request.user_id, user.id)?;
    repo.reject(write_pool, id, user.id, &payload.comment)
        .await?;

    Ok(json!({ "message": "Request rejected" }))
}

fn ensure_admin(user: &User) -> Result<(), AppError> {
    if user.is_admin() {
        Ok(())
    } else {
        Err(AppError::Forbidden("Forbidden".into()))
    }
}

fn parse_optional_user_id(raw: Option<String>) -> Result<Option<UserId>, AppError> {
    raw.map(|value| {
        value
            .parse()
            .map_err(|_| AppError::BadRequest("invalid user_id".into()))
    })
    .transpose()
}

fn ensure_not_self_request(request_user_id: UserId, actor_id: UserId) -> Result<(), AppError> {
    if request_user_id == actor_id {
        Err(AppError::Forbidden(
            "Admins cannot approve or reject their own requests".into(),
        ))
    } else {
        Ok(())
    }
}

fn validate_comment(comment: &str) -> Result<(), AppError> {
    if comment.trim().is_empty() {
        return Err(AppError::BadRequest("comment is required".into()));
    }
    if comment.chars().count() > 500 {
        return Err(AppError::BadRequest(
            "comment must be between 1 and 500 characters".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::user::UserRole;
    use chrono::Utc;

    fn sample_admin() -> User {
        let now = Utc::now();
        User {
            id: UserId::new(),
            username: "admin".to_string(),
            password_hash: "hash".to_string(),
            full_name: "Admin".to_string(),
            email: "admin@example.com".to_string(),
            role: UserRole::Admin,
            is_system_admin: false,
            mfa_secret: None,
            mfa_enabled_at: None,
            password_changed_at: now,
            failed_login_attempts: 0,
            locked_until: None,
            lock_reason: None,
            lockout_count: 0,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn validate_comment_rejects_empty_and_long_values() {
        assert!(matches!(
            validate_comment("   "),
            Err(AppError::BadRequest(_))
        ));
        assert!(matches!(
            validate_comment(&"a".repeat(501)),
            Err(AppError::BadRequest(_))
        ));
    }

    #[test]
    fn ensure_not_self_request_rejects_same_user() {
        let user_id = UserId::new();
        assert!(matches!(
            ensure_not_self_request(user_id, user_id),
            Err(AppError::Forbidden(_))
        ));
    }

    #[test]
    fn parse_optional_user_id_rejects_invalid_value() {
        assert!(matches!(
            parse_optional_user_id(Some("bad-id".to_string())),
            Err(AppError::BadRequest(_))
        ));
    }

    #[test]
    fn ensure_admin_accepts_admin_user() {
        assert!(ensure_admin(&sample_admin()).is_ok());
    }
}
