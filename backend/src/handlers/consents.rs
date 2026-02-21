use axum::{
    extract::{Extension, State},
    http::{header::USER_AGENT, HeaderMap, StatusCode},
    Json,
};
use serde_json::{json, Value};
use uuid::Uuid;
use validator::Validate;

use crate::{
    middleware::request_id::RequestId,
    models::{
        consent_log::{ConsentLog, ConsentLogResponse, RecordConsentPayload},
        user::User,
    },
    repositories::consent_log,
    state::AppState,
    utils::time,
};

const MAX_PURPOSE_LENGTH: usize = 200;
const MAX_POLICY_VERSION_LENGTH: usize = 100;

pub async fn record_consent(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    Json(payload): Json<RecordConsentPayload>,
) -> Result<Json<ConsentLogResponse>, (StatusCode, Json<Value>)> {
    payload.validate().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e.to_string()})),
        )
    })?;
    let purpose = validate_string_field(&payload.purpose, "purpose", MAX_PURPOSE_LENGTH)?;
    let policy_version = validate_string_field(
        &payload.policy_version,
        "policy_version",
        MAX_POLICY_VERSION_LENGTH,
    )?;
    let now = time::now_utc(&state.config.time_zone);

    let log = ConsentLog {
        id: Uuid::new_v4().to_string(),
        user_id: user.id.to_string(),
        purpose,
        policy_version,
        consented_at: now,
        ip: extract_ip(&headers),
        user_agent: extract_user_agent(&headers),
        request_id: Some(request_id.0),
        created_at: now,
    };

    consent_log::insert_consent_log(&state.write_pool, &log)
        .await
        .map_err(|err| {
            tracing::error!(error = %err, "failed to insert consent log");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?;

    Ok(Json(log.into()))
}

pub async fn list_my_consents(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<ConsentLogResponse>>, (StatusCode, Json<Value>)> {
    let user_id = user.id.to_string();
    let logs = consent_log::list_consent_logs_for_user(state.read_pool(), &user_id)
        .await
        .map_err(|err| {
            tracing::error!(error = %err, "failed to list consent logs");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?;

    Ok(Json(
        logs.into_iter().map(ConsentLogResponse::from).collect(),
    ))
}

fn validate_string_field(
    value: &str,
    field: &str,
    max_len: usize,
) -> Result<String, (StatusCode, Json<Value>)> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("{} is required", field)})),
        ));
    }
    if trimmed.chars().count() > max_len {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("{} is too long", field)})),
        ));
    }
    Ok(trimmed.to_string())
}

fn extract_ip(headers: &HeaderMap) -> Option<String> {
    if let Some(value) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        return value
            .split(',')
            .next()
            .map(|ip| ip.trim().to_string())
            .filter(|ip| !ip.is_empty());
    }
    headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .map(|ip| ip.trim().to_string())
        .filter(|ip| !ip.is_empty())
}

fn extract_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|agent| agent.trim().to_string())
        .filter(|agent| !agent.is_empty())
}
