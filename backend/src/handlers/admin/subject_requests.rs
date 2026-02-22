use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::{IntoParams, ToSchema};

use super::requests::validate_decision_comment;
use crate::{
    error::AppError,
    models::{
        request::RequestStatus,
        subject_request::{DataSubjectRequestResponse, DataSubjectRequestType},
        user::User,
    },
    repositories::subject_request::{self, SubjectRequestFilters},
    state::AppState,
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
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(q): Query<SubjectRequestListQuery>,
) -> Result<Json<SubjectRequestListResponse>, (StatusCode, Json<Value>)> {
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }

    let (page, per_page, filters) = validate_list_query(q)?;
    let offset = (page - 1) * per_page;
    let (items, total) =
        subject_request::list_subject_requests(state.read_pool(), &filters, per_page, offset)
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
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(request_id): Path<String>,
    Json(body): Json<DecisionPayload>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }
    validate_decision_comment(&body.comment).map_err(map_app_error)?;
    ensure_pending_request(&state.write_pool, &request_id).await?;
    let now = time::now_utc(&state.config.time_zone);
    let approver_id = user.id.to_string();

    let rows = subject_request::approve_subject_request(
        &state.write_pool,
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
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(request_id): Path<String>,
    Json(body): Json<DecisionPayload>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }
    validate_decision_comment(&body.comment).map_err(map_app_error)?;
    ensure_pending_request(&state.write_pool, &request_id).await?;
    let now = time::now_utc(&state.config.time_zone);
    let approver_id = user.id.to_string();

    let rows = subject_request::reject_subject_request(
        &state.write_pool,
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
    pool: &sqlx::PgPool,
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

    let from = parse_from_datetime(q.from.as_deref()).map_err(bad_request_helper)?;
    let to = parse_to_datetime(q.to.as_deref()).map_err(bad_request_helper)?;

    if let (Some(from), Some(to)) = (from, to) {
        if from > to {
            return Err(bad_request_helper("`from` must be before or equal to `to`"));
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
        _ => Err(bad_request_helper(
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
        _ => Err(bad_request_helper(
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
        AppError::BadRequest(message) => bad_request_helper(&message),
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

fn bad_request_helper(message: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": message })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::holiday::HolidayReason;
    use chrono::{TimeZone, Timelike};

    fn err_message(err: &(StatusCode, Json<Value>)) -> String {
        err.1
             .0
            .get("error")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string()
    }

    #[test]
    fn normalize_filter_trims_and_drops_empty_values() {
        assert_eq!(
            normalize_filter(Some("  user-1  ".to_string())),
            Some("user-1".to_string())
        );
        assert_eq!(normalize_filter(Some("   ".to_string())), None);
        assert_eq!(normalize_filter(None), None);
    }

    #[test]
    fn parse_request_status_accepts_supported_values() {
        assert!(matches!(
            parse_request_status("pending").expect("pending"),
            RequestStatus::Pending
        ));
        assert!(matches!(
            parse_request_status("APPROVED").expect("approved"),
            RequestStatus::Approved
        ));
        assert!(matches!(
            parse_request_status("Rejected").expect("rejected"),
            RequestStatus::Rejected
        ));
        assert!(matches!(
            parse_request_status("cancelled").expect("cancelled"),
            RequestStatus::Cancelled
        ));
    }

    #[test]
    fn parse_request_status_rejects_unknown_value() {
        let err = parse_request_status("unknown").expect_err("invalid status");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
        assert!(err_message(&err).contains("`status`"));
    }

    #[test]
    fn parse_request_type_accepts_supported_values() {
        assert!(matches!(
            parse_request_type("access").expect("access"),
            DataSubjectRequestType::Access
        ));
        assert!(matches!(
            parse_request_type("RECTIFY").expect("rectify"),
            DataSubjectRequestType::Rectify
        ));
        assert!(matches!(
            parse_request_type("Delete").expect("delete"),
            DataSubjectRequestType::Delete
        ));
        assert!(matches!(
            parse_request_type("stop").expect("stop"),
            DataSubjectRequestType::Stop
        ));
    }

    #[test]
    fn parse_request_type_rejects_unknown_value() {
        let err = parse_request_type("archive").expect_err("invalid type");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
        assert!(err_message(&err).contains("`type`"));
    }

    #[test]
    fn parse_datetime_value_supports_rfc3339_and_sql_formats() {
        let rfc = parse_datetime_value("2026-02-04T10:11:12+09:00", true).expect("rfc3339");
        assert_eq!(rfc, Utc.with_ymd_and_hms(2026, 2, 4, 1, 11, 12).unwrap());

        let iso = parse_datetime_value("2026-02-04T10:11:12", true).expect("iso local");
        assert_eq!(iso, Utc.with_ymd_and_hms(2026, 2, 4, 10, 11, 12).unwrap());

        let sql = parse_datetime_value("2026-02-04 10:11:12", true).expect("sql datetime");
        assert_eq!(sql, Utc.with_ymd_and_hms(2026, 2, 4, 10, 11, 12).unwrap());
    }

    #[test]
    fn parse_datetime_value_supports_date_only_for_range_edges() {
        let from = parse_datetime_value("2026-02-04", true).expect("from date");
        assert_eq!(from.time().hour(), 0);
        assert_eq!(from.time().minute(), 0);
        assert_eq!(from.time().second(), 0);

        let to = parse_datetime_value("2026-02-04", false).expect("to date");
        assert_eq!(to.time().hour(), 23);
        assert_eq!(to.time().minute(), 59);
        assert_eq!(to.time().second(), 59);
    }

    #[test]
    fn parse_datetime_value_returns_none_for_invalid_input() {
        assert!(parse_datetime_value("invalid", true).is_none());
        assert!(parse_datetime_value("2026-13-01", true).is_none());
    }

    #[test]
    fn parse_from_and_to_datetime_validate_input() {
        assert!(parse_from_datetime(None).expect("none from").is_none());
        assert!(parse_to_datetime(None).expect("none to").is_none());
        assert!(parse_from_datetime(Some("2026-02-04"))
            .expect("valid from")
            .is_some());
        assert!(parse_to_datetime(Some("2026-02-04"))
            .expect("valid to")
            .is_some());
        assert!(parse_from_datetime(Some("bad")).is_err());
        assert!(parse_to_datetime(Some("bad")).is_err());
    }

    #[test]
    fn validate_list_query_applies_defaults_and_clamps() {
        let (page, per_page, filters) = validate_list_query(SubjectRequestListQuery {
            status: None,
            r#type: None,
            user_id: None,
            from: None,
            to: None,
            page: Some(0),
            per_page: Some(999),
        })
        .expect("valid default query");

        assert_eq!(page, 1);
        assert_eq!(per_page, 100);
        assert!(filters.status.is_none());
        assert!(filters.request_type.is_none());
        assert!(filters.user_id.is_none());
        assert!(filters.from.is_none());
        assert!(filters.to.is_none());
    }

    #[test]
    fn validate_list_query_parses_all_filters() {
        let (page, per_page, filters) = validate_list_query(SubjectRequestListQuery {
            status: Some("approved".to_string()),
            r#type: Some("access".to_string()),
            user_id: Some("  user-123  ".to_string()),
            from: Some("2026-01-01".to_string()),
            to: Some("2026-01-31".to_string()),
            page: Some(3),
            per_page: Some(20),
        })
        .expect("valid query");

        assert_eq!(page, 3);
        assert_eq!(per_page, 20);
        assert!(matches!(filters.status, Some(RequestStatus::Approved)));
        assert!(matches!(
            filters.request_type,
            Some(DataSubjectRequestType::Access)
        ));
        assert_eq!(filters.user_id.as_deref(), Some("user-123"));
        assert_eq!(
            filters.from.expect("from").date_naive(),
            NaiveDate::from_ymd_opt(2026, 1, 1).expect("from date")
        );
        assert_eq!(
            filters.to.expect("to").date_naive(),
            NaiveDate::from_ymd_opt(2026, 1, 31).expect("to date")
        );
    }

    #[test]
    fn validate_list_query_rejects_invalid_filters() {
        let bad_status = validate_list_query(SubjectRequestListQuery {
            status: Some("bad".to_string()),
            r#type: None,
            user_id: None,
            from: None,
            to: None,
            page: None,
            per_page: None,
        })
        .expect_err("invalid status");
        assert_eq!(bad_status.0, StatusCode::BAD_REQUEST);

        let bad_type = validate_list_query(SubjectRequestListQuery {
            status: None,
            r#type: Some("bad".to_string()),
            user_id: None,
            from: None,
            to: None,
            page: None,
            per_page: None,
        })
        .expect_err("invalid type");
        assert_eq!(bad_type.0, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn validate_list_query_rejects_inverted_date_range() {
        let err = validate_list_query(SubjectRequestListQuery {
            status: None,
            r#type: None,
            user_id: None,
            from: Some("2026-02-10".to_string()),
            to: Some("2026-02-01".to_string()),
            page: None,
            per_page: None,
        })
        .expect_err("invalid date range");

        assert_eq!(err.0, StatusCode::BAD_REQUEST);
        assert!(err_message(&err).contains("`from`"));
    }

    #[test]
    fn map_app_error_maps_each_variant() {
        let bad = map_app_error(AppError::BadRequest("bad".to_string()));
        assert_eq!(bad.0, StatusCode::BAD_REQUEST);
        assert_eq!(err_message(&bad), "bad");

        let forbidden = map_app_error(AppError::Forbidden("forbidden".to_string()));
        assert_eq!(forbidden.0, StatusCode::FORBIDDEN);
        assert_eq!(err_message(&forbidden), "forbidden");

        let unauthorized = map_app_error(AppError::Unauthorized("unauthorized".to_string()));
        assert_eq!(unauthorized.0, StatusCode::UNAUTHORIZED);
        assert_eq!(err_message(&unauthorized), "unauthorized");

        let conflict = map_app_error(AppError::Conflict("conflict".to_string()));
        assert_eq!(conflict.0, StatusCode::CONFLICT);
        assert_eq!(err_message(&conflict), "conflict");

        let not_found = map_app_error(AppError::NotFound("missing".to_string()));
        assert_eq!(not_found.0, StatusCode::NOT_FOUND);
        assert_eq!(err_message(&not_found), "missing");

        let validation = map_app_error(AppError::Validation(vec!["field: code".to_string()]));
        assert_eq!(validation.0, StatusCode::BAD_REQUEST);
        assert_eq!(err_message(&validation), "Validation failed");
        assert_eq!(
            validation.1 .0["details"]["errors"][0],
            json!("field: code")
        );

        let internal = map_app_error(AppError::InternalServerError(anyhow::anyhow!("boom")));
        assert_eq!(internal.0, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err_message(&internal), "Internal server error");
    }

    #[test]
    fn bad_request_helper_builds_error_payload() {
        let err = bad_request_helper(HolidayReason::None.label());
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
        assert_eq!(err_message(&err), "working day");
    }
}
