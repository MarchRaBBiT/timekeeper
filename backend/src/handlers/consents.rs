use axum::{
    extract::{Extension, State},
    http::HeaderMap,
    Json,
};

use crate::{
    application::{
        clock::{Clock, SYSTEM_CLOCK},
        http::HttpError,
    },
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
) -> Result<Json<ConsentLogResponse>, HttpError> {
    record_consent_use_case(
        &state.write_pool,
        user.id,
        &request_id,
        &headers,
        payload,
        SYSTEM_CLOCK.now_utc(&state.config.time_zone),
    )
    .await
    .map(Json)
}

pub async fn list_my_consents(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<ConsentLogResponse>>, HttpError> {
    list_user_consents(state.read_pool(), user.id)
        .await
        .map(Json)
}
