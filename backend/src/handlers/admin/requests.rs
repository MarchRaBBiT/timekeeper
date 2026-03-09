use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::application::clock::{Clock, SYSTEM_CLOCK};
use crate::requests::application::admin_requests::paginate_requests as paginate_request_values;

#[cfg(test)]
use crate::requests::application::admin_requests::parse_filter_datetime as parse_filter_datetime_value;

use crate::{
    error::AppError,
    models::user::User,
    requests::application::admin_requests::{
        get_request_detail as get_request_detail_view, list_requests as list_requests_view,
        process_request_decision, validate_decision_comment as validate_decision_comment_value,
        DecisionKind, RequestListParams,
    },
    state::AppState,
};

const MAX_DECISION_COMMENT_LENGTH: usize = 500;

pub type AdminRequestListResponse =
    crate::requests::application::admin_requests::AdminRequestListResponse;
pub type AdminRequestListPageInfo =
    crate::requests::application::admin_requests::AdminRequestListPageInfo;

pub fn paginate_requests(query: &RequestListQuery) -> Result<(i64, i64, i64), AppError> {
    paginate_request_values(query.page, query.per_page)
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct ApprovePayload {
    pub comment: String,
}

pub async fn approve_request(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(request_id): Path<String>,
    Json(body): Json<ApprovePayload>,
) -> Result<Json<crate::application::dto::MessageResponse>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }
    validate_decision_comment_value(&body.comment, MAX_DECISION_COMMENT_LENGTH)?;
    let result = process_request_decision(
        &state.write_pool,
        &request_id,
        user.id,
        &body.comment,
        SYSTEM_CLOCK.now_utc(&state.config.time_zone),
        DecisionKind::Approve,
    )
    .await?;
    Ok(Json(result))
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct RejectPayload {
    pub comment: String,
}

pub async fn reject_request(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(request_id): Path<String>,
    Json(body): Json<RejectPayload>,
) -> Result<Json<crate::application::dto::MessageResponse>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }
    validate_decision_comment_value(&body.comment, MAX_DECISION_COMMENT_LENGTH)?;
    let result = process_request_decision(
        &state.write_pool,
        &request_id,
        user.id,
        &body.comment,
        SYSTEM_CLOCK.now_utc(&state.config.time_zone),
        DecisionKind::Reject,
    )
    .await?;
    Ok(Json(result))
}

#[derive(Debug, Deserialize, Serialize, IntoParams, ToSchema)]
pub struct RequestListQuery {
    pub status: Option<String>,
    #[serde(rename = "type")]
    #[param(value_type = Option<String>)]
    pub r#type: Option<String>,
    pub user_id: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

pub async fn list_requests(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(q): Query<RequestListQuery>,
) -> Result<Json<AdminRequestListResponse>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }
    let response = list_requests_view(
        state.read_pool(),
        RequestListParams {
            status: q.status,
            request_type: q.r#type,
            user_id: q.user_id,
            from: q.from,
            to: q.to,
            page: q.page,
            per_page: q.per_page,
        },
    )
    .await?;

    Ok(Json(response))
}

pub async fn get_request_detail(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(request_id): Path<String>,
) -> Result<Json<crate::requests::application::admin_requests::RequestDetailView>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    let detail = get_request_detail_view(state.read_pool(), &request_id).await?;
    Ok(Json(detail))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn test_validate_decision_comment_accepts_valid_comment() {
        let comment = "This is a valid decision comment";
        assert!(validate_decision_comment_value(comment, MAX_DECISION_COMMENT_LENGTH).is_ok());
    }

    #[test]
    fn test_validate_decision_comment_rejects_empty_comment() {
        let comment = "";
        let result = validate_decision_comment_value(comment, MAX_DECISION_COMMENT_LENGTH);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::BadRequest(_)));
    }

    #[test]
    fn test_validate_decision_comment_rejects_whitespace_only_comment() {
        let comment = "   ";
        let result = validate_decision_comment_value(comment, MAX_DECISION_COMMENT_LENGTH);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::BadRequest(_)));
    }

    #[test]
    fn test_validate_decision_comment_rejects_too_long_comment() {
        let comment = "a".repeat(MAX_DECISION_COMMENT_LENGTH + 1);
        let result = validate_decision_comment_value(&comment, MAX_DECISION_COMMENT_LENGTH);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::BadRequest(_)));
    }

    #[test]
    fn test_validate_decision_comment_accepts_max_length_comment() {
        let comment = "a".repeat(MAX_DECISION_COMMENT_LENGTH);
        assert!(validate_decision_comment_value(&comment, MAX_DECISION_COMMENT_LENGTH).is_ok());
    }

    #[test]
    fn test_paginate_requests_with_defaults() {
        let query = RequestListQuery {
            status: None,
            r#type: None,
            user_id: None,
            from: None,
            to: None,
            page: None,
            per_page: None,
        };
        let result = paginate_request_values(query.page, query.per_page).unwrap();
        assert_eq!(result.0, 1);
        assert_eq!(result.1, 20);
        assert_eq!(result.2, 0);
    }

    #[test]
    fn test_paginate_requests_with_custom_values() {
        let query = RequestListQuery {
            status: None,
            r#type: None,
            user_id: None,
            from: None,
            to: None,
            page: Some(3),
            per_page: Some(50),
        };
        let result = paginate_request_values(query.page, query.per_page).unwrap();
        assert_eq!(result.0, 3);
        assert_eq!(result.1, 50);
        assert_eq!(result.2, 100);
    }

    #[test]
    fn test_paginate_requests_clamps_per_page() {
        let query = RequestListQuery {
            status: None,
            r#type: None,
            user_id: None,
            from: None,
            to: None,
            page: Some(1),
            per_page: Some(200),
        };
        let result = paginate_request_values(query.page, query.per_page).unwrap();
        assert_eq!(result.1, 100);
    }

    #[test]
    fn test_paginate_requests_rejects_zero_page() {
        let query = RequestListQuery {
            status: None,
            r#type: None,
            user_id: None,
            from: None,
            to: None,
            page: Some(0),
            per_page: None,
        };
        let result = paginate_request_values(query.page, query.per_page).unwrap();
        assert_eq!(result.0, 1);
    }

    #[test]
    fn test_paginate_requests_rejects_negative_per_page() {
        let query = RequestListQuery {
            status: None,
            r#type: None,
            user_id: None,
            from: None,
            to: None,
            page: Some(1),
            per_page: Some(-5),
        };
        let result = paginate_request_values(query.page, query.per_page).unwrap();
        assert_eq!(result.1, 1);
    }

    #[test]
    fn test_parse_filter_datetime_parses_rfc3339() {
        let input = "2024-01-15T10:30:00Z";
        let result = parse_filter_datetime_value(input, false);
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_filter_datetime_parses_date_only_start() {
        let input = "2024-01-15";
        let result = parse_filter_datetime_value(input, false);
        assert!(result.is_some());
        let dt = result.unwrap();
        assert_eq!(dt.hour(), 0);
        assert_eq!(dt.minute(), 0);
        assert_eq!(dt.second(), 0);
    }

    #[test]
    fn test_parse_filter_datetime_parses_date_only_end() {
        let input = "2024-01-15";
        let result = parse_filter_datetime_value(input, true);
        assert!(result.is_some());
        let dt = result.unwrap();
        assert_eq!(dt.hour(), 23);
        assert_eq!(dt.minute(), 59);
        assert_eq!(dt.second(), 59);
    }

    #[test]
    fn test_parse_filter_datetime_returns_none_for_invalid() {
        let input = "invalid-date";
        let result = parse_filter_datetime_value(input, false);
        assert!(result.is_none());
    }

    #[test]
    fn test_approve_payload_structure() {
        let payload = ApprovePayload {
            comment: "Approved".to_string(),
        };
        assert_eq!(payload.comment, "Approved");
    }

    #[test]
    fn test_reject_payload_structure() {
        let payload = RejectPayload {
            comment: "Rejected".to_string(),
        };
        assert_eq!(payload.comment, "Rejected");
    }

    #[test]
    fn test_request_list_query_default_values() {
        let query = RequestListQuery {
            status: None,
            r#type: None,
            user_id: None,
            from: None,
            to: None,
            page: None,
            per_page: None,
        };
        assert!(query.status.is_none());
        assert!(query.r#type.is_none());
        assert!(query.page.is_none());
        assert!(query.per_page.is_none());
    }

    #[test]
    fn test_request_list_page_info_structure() {
        let info = AdminRequestListPageInfo {
            page: 1,
            per_page: 20,
        };
        assert_eq!(info.page, 1);
        assert_eq!(info.per_page, 20);
    }

    #[test]
    fn test_request_list_response_structure() {
        let response = AdminRequestListResponse {
            leave_requests: vec![],
            overtime_requests: vec![],
            page_info: AdminRequestListPageInfo {
                page: 1,
                per_page: 20,
            },
        };
        assert!(response.leave_requests.is_empty());
        assert!(response.overtime_requests.is_empty());
        assert_eq!(response.page_info.page, 1);
    }
}
