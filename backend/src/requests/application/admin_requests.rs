use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use serde::Serialize;
use utoipa::ToSchema;

use crate::{
    db::connection::DbPool,
    error::AppError,
    models::{leave_request::LeaveRequestResponse, overtime_request::OvertimeRequestResponse},
    repositories::{
        leave_request::{LeaveRequestRepository, LeaveRequestRepositoryTrait},
        overtime_request::{OvertimeRequestRepository, OvertimeRequestRepositoryTrait},
        request::{RequestListFilters, RequestRepository},
    },
    types::{LeaveRequestId, OvertimeRequestId},
};

#[derive(Debug, Clone)]
pub struct RequestListParams {
    pub status: Option<String>,
    pub request_type: Option<String>,
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

#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "data")]
pub enum RequestDetailView {
    #[serde(rename = "leave")]
    Leave(LeaveRequestResponse),
    #[serde(rename = "overtime")]
    Overtime(OvertimeRequestResponse),
}

pub async fn list_requests(
    read_pool: &DbPool,
    params: RequestListParams,
) -> Result<AdminRequestListResponse, AppError> {
    let (page, per_page, offset) = paginate_requests(params.page, params.per_page)?;

    let type_filter = params
        .request_type
        .as_deref()
        .map(|value| value.to_ascii_lowercase());
    let (include_leave, include_overtime) = match type_filter.as_deref() {
        Some("leave") => (true, false),
        Some("overtime") => (false, true),
        Some("all") => (true, true),
        _ => (true, true),
    };

    let filters = RequestListFilters {
        status: params.status,
        user_id: params.user_id,
        from: params
            .from
            .as_deref()
            .and_then(|value| parse_filter_datetime(value, false)),
        to: params
            .to
            .as_deref()
            .and_then(|value| parse_filter_datetime(value, true)),
    };

    let request_repo = RequestRepository::new();
    let result = request_repo
        .get_requests_with_relations(
            read_pool,
            &filters,
            per_page,
            offset,
            include_leave,
            include_overtime,
        )
        .await?;

    Ok(AdminRequestListResponse {
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
    })
}

pub async fn get_request_detail(
    read_pool: &DbPool,
    request_id: &str,
) -> Result<RequestDetailView, AppError> {
    if let Ok(leave_request_id) = request_id.parse::<LeaveRequestId>() {
        let leave_repo = LeaveRequestRepository::new();
        match leave_repo.find_by_id(read_pool, leave_request_id).await {
            Ok(item) => return Ok(RequestDetailView::Leave(LeaveRequestResponse::from(item))),
            Err(AppError::NotFound(_)) => {}
            Err(err) => return Err(err),
        }
    }

    if let Ok(overtime_request_id) = request_id.parse::<OvertimeRequestId>() {
        let overtime_repo = OvertimeRequestRepository::new();
        match overtime_repo
            .find_by_id(read_pool, overtime_request_id)
            .await
        {
            Ok(item) => {
                return Ok(RequestDetailView::Overtime(OvertimeRequestResponse::from(
                    item,
                )))
            }
            Err(AppError::NotFound(_)) => {}
            Err(err) => return Err(err),
        }
    }

    Err(AppError::NotFound("Request not found".into()))
}

pub fn paginate_requests(
    page: Option<i64>,
    per_page: Option<i64>,
) -> Result<(i64, i64, i64), AppError> {
    let page = page.unwrap_or(1).max(1);
    let per_page = per_page.unwrap_or(20).clamp(1, 100);
    let offset = page
        .checked_sub(1)
        .and_then(|current_page| current_page.checked_mul(per_page))
        .ok_or(AppError::BadRequest("page is too large".into()))?;
    Ok((page, per_page, offset))
}

pub fn parse_filter_datetime(value: &str, end_of_day: bool) -> Option<DateTime<Utc>> {
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
    fn paginate_requests_with_defaults() {
        let result = paginate_requests(None, None).unwrap();
        assert_eq!(result.0, 1);
        assert_eq!(result.1, 20);
        assert_eq!(result.2, 0);
    }

    #[test]
    fn paginate_requests_with_custom_values() {
        let result = paginate_requests(Some(3), Some(50)).unwrap();
        assert_eq!(result.0, 3);
        assert_eq!(result.1, 50);
        assert_eq!(result.2, 100);
    }

    #[test]
    fn paginate_requests_clamps_per_page() {
        let result = paginate_requests(Some(1), Some(200)).unwrap();
        assert_eq!(result.1, 100);
    }

    #[test]
    fn paginate_requests_clamps_zero_page_to_one() {
        let result = paginate_requests(Some(0), None).unwrap();
        assert_eq!(result.0, 1);
    }

    #[test]
    fn paginate_requests_clamps_negative_per_page() {
        let result = paginate_requests(Some(1), Some(-5)).unwrap();
        assert_eq!(result.1, 1);
    }

    #[test]
    fn parse_filter_datetime_parses_rfc3339() {
        let input = "2024-01-15T10:30:00Z";
        let result = parse_filter_datetime(input, false);
        assert!(result.is_some());
    }

    #[test]
    fn parse_filter_datetime_parses_date_only_start() {
        let input = "2024-01-15";
        let result = parse_filter_datetime(input, false);
        assert!(result.is_some());
        let dt = result.unwrap();
        assert_eq!(dt.hour(), 0);
        assert_eq!(dt.minute(), 0);
        assert_eq!(dt.second(), 0);
    }

    #[test]
    fn parse_filter_datetime_parses_date_only_end() {
        let input = "2024-01-15";
        let result = parse_filter_datetime(input, true);
        assert!(result.is_some());
        let dt = result.unwrap();
        assert_eq!(dt.hour(), 23);
        assert_eq!(dt.minute(), 59);
        assert_eq!(dt.second(), 59);
    }

    #[test]
    fn parse_filter_datetime_returns_none_for_invalid() {
        let input = "invalid-date";
        let result = parse_filter_datetime(input, false);
        assert!(result.is_none());
    }

    #[test]
    fn request_detail_view_serializes_like_existing_api() {
        let view = RequestDetailView::Leave(LeaveRequestResponse {
            id: LeaveRequestId::new(),
            user_id: crate::types::UserId::new(),
            leave_type: crate::models::leave_request::LeaveType::Annual,
            start_date: NaiveDate::from_ymd_opt(2026, 1, 1).expect("date"),
            end_date: NaiveDate::from_ymd_opt(2026, 1, 1).expect("date"),
            reason: None,
            status: crate::models::request::RequestStatus::Pending,
            approved_by: None,
            approved_at: None,
            rejected_by: None,
            rejected_at: None,
            cancelled_at: None,
            decision_comment: None,
            created_at: Utc::now(),
        });

        let json = serde_json::to_value(view).expect("serialize detail view");
        assert_eq!(
            json.get("kind").and_then(|value| value.as_str()),
            Some("leave")
        );
        assert!(json.get("data").is_some());
    }
}
