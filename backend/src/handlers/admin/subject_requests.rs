use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;
use utoipa::{IntoParams, ToSchema};

use super::requests::validate_decision_comment;
use crate::{
    config::Config,
    error::AppError,
    models::{
        request::RequestStatus,
        subject_request::{DataSubjectRequestResponse, DataSubjectRequestType},
        user::User,
    },
    repositories::subject_request::{self, SubjectRequestFilters},
    utils::time,
};

const DEFAULT_PAGE: i64 = 1;
const DEFAULT_PER_PAGE: i64 = 25;
const MAX_PER_PAGE: i64 = 100;
const MAX_PAGE: i64 = 1_000;

#[derive(Debug, Deserialize, Serialize, IntoParams, ToSchema)]
pub struct SubjectRequestListQuery {
    pub status: Option<String>,
    #[serde(rename = "type")]
    pub r#type: Option<String>,
    pub user_id: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SubjectRequestListResponse {
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub items: Vec<DataSubjectRequestResponse>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct DecisionPayload {
    pub comment: String,
}

pub async fn list_subject_requests(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Query(q): Query<SubjectRequestListQuery>,
) -> Result<Json<SubjectRequestListResponse>, (StatusCode, Json<Value>)> {
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }

    let (page, per_page, filters) = validate_list_query(q)?;
    let offset = (page - 1) * per_page;
    let (items, total) = subject_request::list_subject_requests(&pool, &filters, per_page, offset)
        .await
        .map_err(|err| {
            tracing::error!(error = %err, "failed to list subject requests");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?;

    Ok(Json(SubjectRequestListResponse {
        page,
        per_page,
        total,
        items: items
            .into_iter()
            .map(DataSubjectRequestResponse::from)
            .collect(),
    }))
}

pub async fn approve_subject_request(
    State((pool, config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Path(request_id): Path<String>,
    Json(body): Json<DecisionPayload>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }
    validate_decision_comment(&body.comment).map_err(map_app_error)?;
    ensure_pending_request(&pool, &request_id).await?;
    let now = time::now_utc(&config.time_zone);
    let approver_id = user.id.to_string();

    let rows = subject_request::approve_subject_request(
        &pool,
        &request_id,
        &approver_id,
        &body.comment,
        now,
    )
    .await
    .map_err(|err| {
        tracing::error!(error = %err, "failed to approve subject request");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        )
    })?;

    if rows == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Request not found or already processed"})),
        ));
    }

    Ok(Json(json!({"message": "Subject request approved"})))
}

pub async fn reject_subject_request(
    State((pool, config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Path(request_id): Path<String>,
    Json(body): Json<DecisionPayload>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }
    validate_decision_comment(&body.comment).map_err(map_app_error)?;
    ensure_pending_request(&pool, &request_id).await?;
    let now = time::now_utc(&config.time_zone);
    let approver_id = user.id.to_string();

    let rows = subject_request::reject_subject_request(
        &pool,
        &request_id,
        &approver_id,
        &body.comment,
        now,
    )
    .await
    .map_err(|err| {
        tracing::error!(error = %err, "failed to reject subject request");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        )
    })?;

    if rows == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Request not found or already processed"})),
        ));
    }

    Ok(Json(json!({"message": "Subject request rejected"})))
}

async fn ensure_pending_request(
    pool: &PgPool,
    request_id: &str,
) -> Result<(), (StatusCode, Json<Value>)> {
    let existing = subject_request::fetch_subject_request(pool, request_id)
        .await
        .map_err(|err| {
            tracing::error!(error = %err, "failed to fetch subject request");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?;

    match existing {
        Some(request) if matches!(request.status, RequestStatus::Pending) => Ok(()),
        _ => Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Request not found or already processed"})),
        )),
    }
}

fn validate_list_query(
    q: SubjectRequestListQuery,
) -> Result<(i64, i64, SubjectRequestFilters), (StatusCode, Json<Value>)> {
    let page = q.page.unwrap_or(DEFAULT_PAGE).clamp(1, MAX_PAGE);
    let per_page = q
        .per_page
        .unwrap_or(DEFAULT_PER_PAGE)
        .clamp(1, MAX_PER_PAGE);

    let from = parse_from_datetime(q.from.as_deref()).map_err(bad_request)?;
    let to = parse_to_datetime(q.to.as_deref()).map_err(bad_request)?;

    if let (Some(from), Some(to)) = (from, to) {
        if from > to {
            return Err(bad_request("`from` must be before or equal to `to`"));
        }
    }

    let status = q.status.as_deref().map(parse_request_status).transpose()?;
    let request_type = q.r#type.as_deref().map(parse_request_type).transpose()?;

    let user_id = normalize_filter(q.user_id);

    Ok((
        page,
        per_page,
        SubjectRequestFilters {
            status,
            request_type,
            user_id,
            from,
            to,
        },
    ))
}

fn normalize_filter(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn parse_request_status(value: &str) -> Result<RequestStatus, (StatusCode, Json<Value>)> {
    match value.to_ascii_lowercase().as_str() {
        "pending" => Ok(RequestStatus::Pending),
        "approved" => Ok(RequestStatus::Approved),
        "rejected" => Ok(RequestStatus::Rejected),
        "cancelled" => Ok(RequestStatus::Cancelled),
        _ => Err(bad_request(
            "`status` must be pending, approved, rejected, or cancelled",
        )),
    }
}

fn parse_request_type(value: &str) -> Result<DataSubjectRequestType, (StatusCode, Json<Value>)> {
    match value.to_ascii_lowercase().as_str() {
        "access" => Ok(DataSubjectRequestType::Access),
        "rectify" => Ok(DataSubjectRequestType::Rectify),
        "delete" => Ok(DataSubjectRequestType::Delete),
        "stop" => Ok(DataSubjectRequestType::Stop),
        _ => Err(bad_request(
            "`type` must be access, rectify, delete, or stop",
        )),
    }
}

fn parse_from_datetime(raw: Option<&str>) -> Result<Option<DateTime<Utc>>, &'static str> {
    match raw {
        Some(value) => parse_datetime_value(value, true)
            .ok_or("`from` must be a valid datetime (RFC3339 or YYYY-MM-DD)")
            .map(Some),
        None => Ok(None),
    }
}

fn parse_to_datetime(raw: Option<&str>) -> Result<Option<DateTime<Utc>>, &'static str> {
    match raw {
        Some(value) => parse_datetime_value(value, false)
            .ok_or("`to` must be a valid datetime (RFC3339 or YYYY-MM-DD)")
            .map(Some),
        None => Ok(None),
    }
}

fn parse_datetime_value(value: &str, is_start: bool) -> Option<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        return Some(dt.with_timezone(&Utc));
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S") {
        return Some(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S") {
        return Some(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
    }
    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        let time = if is_start {
            NaiveTime::from_hms_opt(0, 0, 0)
        } else {
            NaiveTime::from_hms_opt(23, 59, 59)
        }?;
        return Some(DateTime::<Utc>::from_naive_utc_and_offset(
            NaiveDateTime::new(date, time),
            Utc,
        ));
    }
    None
}

fn map_app_error(err: AppError) -> (StatusCode, Json<Value>) {
    match err {
        AppError::BadRequest(message) => bad_request(&message),
        AppError::Forbidden(message) => (StatusCode::FORBIDDEN, Json(json!({ "error": message }))),
        AppError::Unauthorized(message) => {
            (StatusCode::UNAUTHORIZED, Json(json!({ "error": message })))
        }
        AppError::Conflict(message) => (StatusCode::CONFLICT, Json(json!({ "error": message }))),
        AppError::NotFound(message) => (StatusCode::NOT_FOUND, Json(json!({ "error": message }))),
        AppError::Validation(errors) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Validation failed", "details": { "errors": errors } })),
        ),
        AppError::InternalServerError(err) => {
            tracing::error!(error = %err, "internal server error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal server error" })),
            )
        }
    }
}

fn bad_request(message: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": message })))
}
