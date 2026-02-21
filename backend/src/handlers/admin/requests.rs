use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::str::FromStr;
use utoipa::{IntoParams, ToSchema};

use crate::{
    error::AppError,
    models::{
        leave_request::LeaveRequestResponse, overtime_request::OvertimeRequestResponse, user::User,
    },
    repositories::{
        leave_request::{LeaveRequestRepository, LeaveRequestRepositoryTrait},
        overtime_request::{OvertimeRequestRepository, OvertimeRequestRepositoryTrait},
        request::{RequestListFilters, RequestRepository, RequestStatusUpdate},
    },
    state::AppState,
    types::{LeaveRequestId, OvertimeRequestId},
    utils::time,
};

const MAX_DECISION_COMMENT_LENGTH: usize = 500;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct ApprovePayload {
    pub comment: String,
}

pub async fn approve_request(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(request_id): Path<String>,
    Json(body): Json<ApprovePayload>,
) -> Result<Json<Value>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }
    ensure_not_self_request(&state, &request_id, user.id).await?;
    validate_decision_comment(&body.comment)?;
    let approver_id = user.id;
    let comment = body.comment;
    let now_utc = time::now_utc(&state.config.time_zone);
    let request_repo = RequestRepository::new();
    if request_repo
        .update_request_status(
            &state.write_pool,
            &request_id,
            RequestStatusUpdate::Approve {
                approver_id,
                comment: &comment,
                timestamp: now_utc,
            },
        )
        .await?
    {
        return Ok(Json(json!({"message": "Request approved"})));
    }

    Err(AppError::NotFound(
        "Request not found or already processed".into(),
    ))
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
) -> Result<Json<Value>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }
    ensure_not_self_request(&state, &request_id, user.id).await?;
    validate_decision_comment(&body.comment)?;
    let approver_id = user.id;
    let comment = body.comment;
    let now_utc = time::now_utc(&state.config.time_zone);
    let request_repo = RequestRepository::new();
    if request_repo
        .update_request_status(
            &state.write_pool,
            &request_id,
            RequestStatusUpdate::Reject {
                approver_id,
                comment: &comment,
                timestamp: now_utc,
            },
        )
        .await?
    {
        return Ok(Json(json!({"message": "Request rejected"})));
    }

    Err(AppError::NotFound(
        "Request not found or already processed".into(),
    ))
}

async fn ensure_not_self_request(
    state: &AppState,
    request_id: &str,
    actor_id: crate::types::UserId,
) -> Result<(), AppError> {
    if let Ok(leave_request_id) = LeaveRequestId::from_str(request_id) {
        let leave_repo = LeaveRequestRepository::new();
        if let Ok(request) = leave_repo
            .find_by_id(&state.write_pool, leave_request_id)
            .await
        {
            if request.user_id == actor_id {
                return Err(AppError::Forbidden(
                    "Admins cannot approve or reject their own requests".into(),
                ));
            }
            return Ok(());
        }
    }

    if let Ok(overtime_request_id) = OvertimeRequestId::from_str(request_id) {
        let overtime_repo = OvertimeRequestRepository::new();
        if let Ok(request) = overtime_repo
            .find_by_id(&state.write_pool, overtime_request_id)
            .await
        {
            if request.user_id == actor_id {
                return Err(AppError::Forbidden(
                    "Admins cannot approve or reject their own requests".into(),
                ));
            }
        }
    }

    Ok(())
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

#[derive(Debug, Serialize, ToSchema)]
pub struct AdminRequestListResponse {
    pub leave_requests: Vec<LeaveRequestResponse>,
    pub overtime_requests: Vec<OvertimeRequestResponse>,
    pub page_info: AdminRequestListPageInfo,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AdminRequestListPageInfo {
    pub page: i64,
    pub per_page: i64,
}

pub async fn list_requests(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(q): Query<RequestListQuery>,
) -> Result<Json<AdminRequestListResponse>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }
    let (page, per_page, offset) = paginate_requests(&q)?;

    let type_filter = q.r#type.as_deref().map(|s| s.to_ascii_lowercase());
    let (include_leave, include_overtime) = match type_filter.as_deref() {
        Some("leave") => (true, false),
        Some("overtime") => (false, true),
        Some("all") => (true, true),
        _ => (true, true),
    };

    let filters = RequestListFilters {
        status: q.status,
        user_id: q.user_id,
        from: q
            .from
            .as_deref()
            .and_then(|value| parse_filter_datetime(value, false)),
        to: q
            .to
            .as_deref()
            .and_then(|value| parse_filter_datetime(value, true)),
    };

    let request_repo = RequestRepository::new();
    let result = request_repo
        .get_requests_with_relations(
            state.read_pool(),
            &filters,
            per_page,
            offset,
            include_leave,
            include_overtime,
        )
        .await?;

    Ok(Json(AdminRequestListResponse {
        leave_requests: result
            .leave_requests
            .into_iter()
            .map(LeaveRequestResponse::from)
            .collect::<Vec<_>>(),
        overtime_requests: result
            .overtime_requests
            .into_iter()
            .map(OvertimeRequestResponse::from)
            .collect::<Vec<_>>(),
        page_info: AdminRequestListPageInfo { page, per_page },
    }))
}

pub fn paginate_requests(q: &RequestListQuery) -> Result<(i64, i64, i64), AppError> {
    let page = q.page.unwrap_or(1).max(1);
    let per_page = q.per_page.unwrap_or(20).clamp(1, 100);
    let offset = page
        .checked_sub(1)
        .and_then(|p| p.checked_mul(per_page))
        .ok_or(AppError::BadRequest("page is too large".into()))?;
    Ok((page, per_page, offset))
}

pub(crate) fn validate_decision_comment(comment: &str) -> Result<(), AppError> {
    if comment.trim().is_empty() {
        return Err(AppError::BadRequest("comment is required".into()));
    }

    if comment.chars().count() > MAX_DECISION_COMMENT_LENGTH {
        return Err(AppError::BadRequest(format!(
            "comment must be between 1 and {} characters",
            MAX_DECISION_COMMENT_LENGTH
        )));
    }

    Ok(())
}

pub async fn get_request_detail(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(request_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    // Try as leave request
    if let Ok(leave_request_id) = LeaveRequestId::from_str(&request_id) {
        let leave_repo = LeaveRequestRepository::new();
        match leave_repo
            .find_by_id(state.read_pool(), leave_request_id)
            .await
        {
            Ok(item) => {
                return Ok(Json(
                    json!({"kind":"leave","data": LeaveRequestResponse::from(item)}),
                ));
            }
            Err(AppError::NotFound(_)) => {}
            Err(err) => return Err(err),
        }
    }

    // Try as overtime request
    if let Ok(overtime_request_id) = OvertimeRequestId::from_str(&request_id) {
        let overtime_repo = OvertimeRequestRepository::new();
        match overtime_repo
            .find_by_id(state.read_pool(), overtime_request_id)
            .await
        {
            Ok(item) => {
                return Ok(Json(
                    json!({"kind":"overtime","data": OvertimeRequestResponse::from(item)}),
                ));
            }
            Err(AppError::NotFound(_)) => {}
            Err(err) => return Err(err),
        }
    }

    Err(AppError::NotFound("Request not found".into()))
}

fn parse_filter_datetime(value: &str, end_of_day: bool) -> Option<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        return Some(dt.with_timezone(&Utc));
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S") {
        return Some(Utc.from_utc_datetime(&dt));
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S") {
        return Some(Utc.from_utc_datetime(&dt));
    }
    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        let dt = if end_of_day {
            date.and_hms_opt(23, 59, 59)?
        } else {
            date.and_hms_opt(0, 0, 0)?
        };
        return Some(Utc.from_utc_datetime(&dt));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn test_validate_decision_comment_accepts_valid_comment() {
        let comment = "This is a valid decision comment";
        assert!(validate_decision_comment(comment).is_ok());
    }

    #[test]
    fn test_validate_decision_comment_rejects_empty_comment() {
        let comment = "";
        let result = validate_decision_comment(comment);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::BadRequest(_)));
    }

    #[test]
    fn test_validate_decision_comment_rejects_whitespace_only_comment() {
        let comment = "   ";
        let result = validate_decision_comment(comment);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::BadRequest(_)));
    }

    #[test]
    fn test_validate_decision_comment_rejects_too_long_comment() {
        let comment = "a".repeat(MAX_DECISION_COMMENT_LENGTH + 1);
        let result = validate_decision_comment(&comment);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::BadRequest(_)));
    }

    #[test]
    fn test_validate_decision_comment_accepts_max_length_comment() {
        let comment = "a".repeat(MAX_DECISION_COMMENT_LENGTH);
        assert!(validate_decision_comment(&comment).is_ok());
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
        let result = paginate_requests(&query).unwrap();
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
        let result = paginate_requests(&query).unwrap();
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
        let result = paginate_requests(&query).unwrap();
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
        let result = paginate_requests(&query).unwrap();
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
        let result = paginate_requests(&query).unwrap();
        assert_eq!(result.1, 1);
    }

    #[test]
    fn test_parse_filter_datetime_parses_rfc3339() {
        let input = "2024-01-15T10:30:00Z";
        let result = parse_filter_datetime(input, false);
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_filter_datetime_parses_date_only_start() {
        let input = "2024-01-15";
        let result = parse_filter_datetime(input, false);
        assert!(result.is_some());
        let dt = result.unwrap();
        assert_eq!(dt.hour(), 0);
        assert_eq!(dt.minute(), 0);
        assert_eq!(dt.second(), 0);
    }

    #[test]
    fn test_parse_filter_datetime_parses_date_only_end() {
        let input = "2024-01-15";
        let result = parse_filter_datetime(input, true);
        assert!(result.is_some());
        let dt = result.unwrap();
        assert_eq!(dt.hour(), 23);
        assert_eq!(dt.minute(), 59);
        assert_eq!(dt.second(), 59);
    }

    #[test]
    fn test_parse_filter_datetime_returns_none_for_invalid() {
        let input = "invalid-date";
        let result = parse_filter_datetime(input, false);
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
