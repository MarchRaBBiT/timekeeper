use axum::http::{header::USER_AGENT, HeaderMap};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    application::http::{bad_request, internal_server_error, HttpError},
    middleware::request_id::RequestId,
    models::consent_log::{ConsentLog, ConsentLogResponse, RecordConsentPayload},
    repositories::consent_log,
    types::UserId,
};

const MAX_PURPOSE_LENGTH: usize = 200;
const MAX_POLICY_VERSION_LENGTH: usize = 100;

pub async fn record_consent(
    write_pool: &sqlx::PgPool,
    user_id: UserId,
    request_id: &RequestId,
    headers: &HeaderMap,
    payload: RecordConsentPayload,
    now: chrono::DateTime<Utc>,
) -> Result<ConsentLogResponse, HttpError> {
    let purpose = validate_string_field(&payload.purpose, "purpose", MAX_PURPOSE_LENGTH)?;
    let policy_version = validate_string_field(
        &payload.policy_version,
        "policy_version",
        MAX_POLICY_VERSION_LENGTH,
    )?;

    let log = ConsentLog {
        id: Uuid::new_v4().to_string(),
        user_id: user_id.to_string(),
        purpose,
        policy_version,
        consented_at: now,
        ip: extract_ip(headers),
        user_agent: extract_user_agent(headers),
        request_id: Some(request_id.0.clone()),
        created_at: now,
    };

    consent_log::insert_consent_log(write_pool, &log)
        .await
        .map_err(|err| {
            tracing::error!(error = %err, "failed to insert consent log");
            internal_server_error(&err.into())
        })?;

    Ok(log.into())
}

pub async fn list_user_consents(
    read_pool: &sqlx::PgPool,
    user_id: UserId,
) -> Result<Vec<ConsentLogResponse>, HttpError> {
    let logs = consent_log::list_consent_logs_for_user(read_pool, &user_id.to_string())
        .await
        .map_err(|err| {
            tracing::error!(error = %err, "failed to list consent logs");
            internal_server_error(&err.into())
        })?;

    Ok(logs.into_iter().map(ConsentLogResponse::from).collect())
}

pub fn validate_string_field(
    value: &str,
    field: &str,
    max_len: usize,
) -> Result<String, HttpError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(bad_request(format!("{field} is required")));
    }
    if trimmed.chars().count() > max_len {
        return Err(bad_request(format!("{field} is too long")));
    }
    Ok(trimmed.to_string())
}

pub fn extract_ip(headers: &HeaderMap) -> Option<String> {
    if let Some(value) = headers
        .get("x-forwarded-for")
        .and_then(|header| header.to_str().ok())
    {
        return value
            .split(',')
            .next()
            .map(|ip| ip.trim().to_string())
            .filter(|ip| !ip.is_empty());
    }
    headers
        .get("x-real-ip")
        .and_then(|header| header.to_str().ok())
        .map(|ip| ip.trim().to_string())
        .filter(|ip| !ip.is_empty())
}

pub fn extract_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(USER_AGENT)
        .and_then(|header| header.to_str().ok())
        .map(|agent| agent.trim().to_string())
        .filter(|agent| !agent.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::dto::ErrorResponse;
    use axum::http::StatusCode;

    #[test]
    fn validate_string_field_rejects_empty_and_long_values() {
        assert!(validate_string_field("   ", "purpose", 10).is_err());
        assert!(validate_string_field(&"a".repeat(11), "purpose", 10).is_err());
    }

    #[test]
    fn validate_string_field_uses_shared_error_payload() {
        let err = validate_string_field("   ", "purpose", 10).expect_err("empty should fail");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
        assert_eq!(err.1 .0, ErrorResponse::new("purpose is required"));
    }
}
