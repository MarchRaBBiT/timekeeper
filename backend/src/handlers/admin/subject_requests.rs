use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::{IntoParams, ToSchema};

use crate::{
    admin::application::http_errors::map_app_error,
    models::{subject_request::DataSubjectRequestResponse, user::User},
    requests::application::admin_requests::validate_decision_comment as validate_decision_comment_value,
    requests::application::admin_subject_requests::{
        list_subject_requests as list_subject_requests_view, process_subject_request_decision,
        DecisionKind, SubjectRequestListParams,
    },
    state::AppState,
    utils::time,
};

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

    let response = list_subject_requests_view(
        state.read_pool(),
        SubjectRequestListParams {
            status: q.status,
            request_type: q.r#type,
            user_id: q.user_id,
            from: q.from,
            to: q.to,
            page: q.page,
            per_page: q.per_page,
        },
    )
    .await
    .map_err(map_app_error)?;

    Ok(Json(SubjectRequestListResponse {
        page: response.page,
        per_page: response.per_page,
        total: response.total,
        items: response.items,
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
    validate_decision_comment_value(&body.comment, 500).map_err(map_app_error)?;
    let result = process_subject_request_decision(
        &state.write_pool,
        &request_id,
        user.id,
        &body.comment,
        time::now_utc(&state.config.time_zone),
        DecisionKind::Approve,
    )
    .await
    .map_err(map_app_error)?;

    Ok(Json(json!(result)))
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
    validate_decision_comment_value(&body.comment, 500).map_err(map_app_error)?;
    let result = process_subject_request_decision(
        &state.write_pool,
        &request_id,
        user.id,
        &body.comment,
        time::now_utc(&state.config.time_zone),
        DecisionKind::Reject,
    )
    .await
    .map_err(map_app_error)?;

    Ok(Json(json!(result)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        admin::application::http_errors::bad_request,
        models::{request::RequestStatus, subject_request::DataSubjectRequestType},
        requests::application::admin_subject_requests::{
            normalize_filter, parse_datetime_value, parse_from_datetime, parse_request_status,
            parse_request_type, parse_to_datetime, validate_list_params, SubjectRequestListParams,
        },
        services::holiday::HolidayReason,
    };
    use chrono::{NaiveDate, TimeZone, Timelike, Utc};

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
        let err = map_app_error(err);
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
        let err = map_app_error(err);
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
        let (page, per_page, filters) = validate_list_params(SubjectRequestListParams {
            status: None,
            request_type: None,
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
        let (page, per_page, filters) = validate_list_params(SubjectRequestListParams {
            status: Some("approved".to_string()),
            request_type: Some("access".to_string()),
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
        let bad_status = validate_list_params(SubjectRequestListParams {
            status: Some("bad".to_string()),
            request_type: None,
            user_id: None,
            from: None,
            to: None,
            page: None,
            per_page: None,
        })
        .expect_err("invalid status");
        assert_eq!(map_app_error(bad_status).0, StatusCode::BAD_REQUEST);

        let bad_type = validate_list_params(SubjectRequestListParams {
            status: None,
            request_type: Some("bad".to_string()),
            user_id: None,
            from: None,
            to: None,
            page: None,
            per_page: None,
        })
        .expect_err("invalid type");
        assert_eq!(map_app_error(bad_type).0, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn validate_list_query_rejects_inverted_date_range() {
        let err = validate_list_params(SubjectRequestListParams {
            status: None,
            request_type: None,
            user_id: None,
            from: Some("2026-02-10".to_string()),
            to: Some("2026-02-01".to_string()),
            page: None,
            per_page: None,
        })
        .expect_err("invalid date range");

        let err = map_app_error(err);
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
        assert!(err_message(&err).contains("`from`"));
    }

    #[test]
    fn bad_request_helper_builds_error_payload() {
        let err = bad_request(HolidayReason::None.label());
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
        assert_eq!(err_message(&err), "working day");
    }
}
