use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};

use crate::{
    models::{
        subject_request::{
            CreateDataSubjectRequest, DataSubjectRequest, DataSubjectRequestResponse,
        },
        user::User,
    },
    repositories::subject_request,
    state::AppState,
    utils::time,
};

const MAX_DETAILS_LENGTH: usize = 2000;

pub async fn create_subject_request(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<CreateDataSubjectRequest>,
) -> Result<Json<DataSubjectRequestResponse>, (StatusCode, Json<Value>)> {
    let details = validate_details(payload.details)?;
    let now = time::now_utc(&state.config.time_zone);
    let user_id = user.id.to_string();
    let request = DataSubjectRequest::new(user_id, payload.request_type, details, now);

    subject_request::insert_subject_request(&state.write_pool, &request)
        .await
        .map_err(|err| {
            tracing::error!(error = %err, "failed to create subject request");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?;

    Ok(Json(DataSubjectRequestResponse::from(request)))
}

pub async fn list_my_subject_requests(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<DataSubjectRequestResponse>>, (StatusCode, Json<Value>)> {
    let user_id = user.id.to_string();
    let requests = subject_request::list_subject_requests_by_user(state.read_pool(), &user_id)
        .await
        .map_err(|err| {
            tracing::error!(error = %err, "failed to list subject requests");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?;

    Ok(Json(
        requests
            .into_iter()
            .map(DataSubjectRequestResponse::from)
            .collect(),
    ))
}

pub async fn cancel_subject_request(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(request_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let now = time::now_utc(&state.config.time_zone);
    let user_id = user.id.to_string();
    let rows =
        subject_request::cancel_subject_request(&state.write_pool, &request_id, &user_id, now)
            .await
            .map_err(|err| {
                tracing::error!(error = %err, "failed to cancel subject request");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": "Database error"})),
                )
            })?;

    if rows == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Request not found or not cancellable"})),
        ));
    }

    Ok(Json(json!({"id": request_id, "status": "cancelled"})))
}

fn validate_details(details: Option<String>) -> Result<Option<String>, (StatusCode, Json<Value>)> {
    if let Some(details) = details {
        let trimmed = details.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        if trimmed.chars().count() > MAX_DETAILS_LENGTH {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "details is too long"})),
            ));
        }
        return Ok(Some(trimmed.to_string()));
    }
    Ok(None)
}
