use axum::{
    extract::{Extension, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde_json::Value;

use crate::{
    identity::application::consents::{
        list_user_consents, record_consent as record_consent_use_case,
    },
    middleware::request_id::RequestId,
    models::{
        consent_log::{ConsentLogResponse, RecordConsentPayload},
        user::User,
    },
    state::AppState,
};

pub use crate::identity::application::consents::{
    extract_ip, extract_user_agent, validate_string_field,
};

pub async fn record_consent(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    Json(payload): Json<RecordConsentPayload>,
) -> Result<Json<ConsentLogResponse>, (StatusCode, Json<Value>)> {
    record_consent_use_case(
        &state.write_pool,
        user.id,
        &request_id,
        &headers,
        payload,
        crate::utils::time::now_utc(&state.config.time_zone),
    )
    .await
    .map(Json)
    .map_err(|(status, body)| (status, Json(body.0)))
}

pub async fn list_my_consents(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<ConsentLogResponse>>, (StatusCode, Json<Value>)> {
    list_user_consents(state.read_pool(), user.id)
        .await
        .map(Json)
        .map_err(|(status, body)| (status, Json(body.0)))
}
