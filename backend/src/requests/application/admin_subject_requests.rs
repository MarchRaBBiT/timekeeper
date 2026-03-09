use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde::Serialize;

use crate::{
    application::dto::MessageResponse,
    error::AppError,
    models::{
        request::RequestStatus,
        subject_request::{DataSubjectRequestResponse, DataSubjectRequestType},
    },
    repositories::subject_request::{self, SubjectRequestFilters},
    types::UserId,
};

const DEFAULT_PAGE: i64 = 1;
const DEFAULT_PER_PAGE: i64 = 25;
const MAX_PER_PAGE: i64 = 100;
const MAX_PAGE: i64 = 1_000;

#[derive(Debug, Clone)]
pub struct SubjectRequestListParams {
    pub status: Option<String>,
    pub request_type: Option<String>,
    pub user_id: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct SubjectRequestListView {
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub items: Vec<DataSubjectRequestResponse>,
}

#[derive(Debug, Clone, Copy)]
pub enum DecisionKind {
    Approve,
    Reject,
}

pub async fn list_subject_requests(
    pool: &sqlx::PgPool,
    params: SubjectRequestListParams,
) -> Result<SubjectRequestListView, AppError> {
    let (page, per_page, filters) = validate_list_params(params)?;
    let offset = (page - 1) * per_page;
    let (items, total) = subject_request::list_subject_requests(pool, &filters, per_page, offset)
        .await
        .map_err(|err| AppError::InternalServerError(err.into()))?;

    Ok(SubjectRequestListView {
        page,
        per_page,
        total,
        items: items
            .into_iter()
            .map(DataSubjectRequestResponse::from)
            .collect(),
    })
}

pub async fn process_subject_request_decision(
    pool: &sqlx::PgPool,
    request_id: &str,
    actor_id: UserId,
    comment: &str,
    now: DateTime<Utc>,
    kind: DecisionKind,
) -> Result<MessageResponse, AppError> {
    ensure_pending_request(pool, request_id).await?;
    let actor_id = actor_id.to_string();

    let rows = match kind {
        DecisionKind::Approve => {
            subject_request::approve_subject_request(pool, request_id, &actor_id, comment, now)
                .await
        }
        DecisionKind::Reject => {
            subject_request::reject_subject_request(pool, request_id, &actor_id, comment, now).await
        }
    }
    .map_err(|err| AppError::InternalServerError(err.into()))?;

    if rows == 0 {
        return Err(AppError::NotFound(
            "Request not found or already processed".into(),
        ));
    }

    Ok(MessageResponse::new(match kind {
        DecisionKind::Approve => "Subject request approved",
        DecisionKind::Reject => "Subject request rejected",
    }))
}

pub fn validate_list_params(
    params: SubjectRequestListParams,
) -> Result<(i64, i64, SubjectRequestFilters), AppError> {
    let page = params.page.unwrap_or(DEFAULT_PAGE).clamp(1, MAX_PAGE);
    let per_page = params
        .per_page
        .unwrap_or(DEFAULT_PER_PAGE)
        .clamp(1, MAX_PER_PAGE);

    let from = parse_from_datetime(params.from.as_deref())?;
    let to = parse_to_datetime(params.to.as_deref())?;

    if let (Some(from), Some(to)) = (from, to) {
        if from > to {
            return Err(AppError::BadRequest(
                "`from` must be before or equal to `to`".into(),
            ));
        }
    }

    let status = params
        .status
        .as_deref()
        .map(parse_request_status)
        .transpose()?;
    let request_type = params
        .request_type
        .as_deref()
        .map(parse_request_type)
        .transpose()?;

    Ok((
        page,
        per_page,
        SubjectRequestFilters {
            status,
            request_type,
            user_id: normalize_filter(params.user_id),
            from,
            to,
        },
    ))
}

pub async fn ensure_pending_request(pool: &sqlx::PgPool, request_id: &str) -> Result<(), AppError> {
    let existing = subject_request::fetch_subject_request(pool, request_id)
        .await
        .map_err(|err| AppError::InternalServerError(err.into()))?;

    match existing {
        Some(request) if matches!(request.status, RequestStatus::Pending) => Ok(()),
        _ => Err(AppError::NotFound(
            "Request not found or already processed".into(),
        )),
    }
}

pub fn normalize_filter(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn parse_request_status(value: &str) -> Result<RequestStatus, AppError> {
    match value.to_ascii_lowercase().as_str() {
        "pending" => Ok(RequestStatus::Pending),
        "approved" => Ok(RequestStatus::Approved),
        "rejected" => Ok(RequestStatus::Rejected),
        "cancelled" => Ok(RequestStatus::Cancelled),
        _ => Err(AppError::BadRequest(
            "`status` must be pending, approved, rejected, or cancelled".into(),
        )),
    }
}

pub fn parse_request_type(value: &str) -> Result<DataSubjectRequestType, AppError> {
    match value.to_ascii_lowercase().as_str() {
        "access" => Ok(DataSubjectRequestType::Access),
        "rectify" => Ok(DataSubjectRequestType::Rectify),
        "delete" => Ok(DataSubjectRequestType::Delete),
        "stop" => Ok(DataSubjectRequestType::Stop),
        _ => Err(AppError::BadRequest(
            "`type` must be access, rectify, delete, or stop".into(),
        )),
    }
}

pub fn parse_from_datetime(raw: Option<&str>) -> Result<Option<DateTime<Utc>>, AppError> {
    match raw {
        Some(value) => parse_datetime_value(value, true)
            .ok_or_else(|| {
                AppError::BadRequest(
                    "`from` must be a valid datetime (RFC3339 or YYYY-MM-DD)".into(),
                )
            })
            .map(Some),
        None => Ok(None),
    }
}

pub fn parse_to_datetime(raw: Option<&str>) -> Result<Option<DateTime<Utc>>, AppError> {
    match raw {
        Some(value) => parse_datetime_value(value, false)
            .ok_or_else(|| {
                AppError::BadRequest("`to` must be a valid datetime (RFC3339 or YYYY-MM-DD)".into())
            })
            .map(Some),
        None => Ok(None),
    }
}

pub fn parse_datetime_value(value: &str, is_start: bool) -> Option<DateTime<Utc>> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Timelike};

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
        assert!(matches!(
            parse_request_status("unknown"),
            Err(AppError::BadRequest(_))
        ));
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
        assert!(matches!(
            parse_request_type("archive"),
            Err(AppError::BadRequest(_))
        ));
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
        assert!(matches!(
            parse_from_datetime(Some("bad")),
            Err(AppError::BadRequest(_))
        ));
        assert!(matches!(
            parse_to_datetime(Some("bad")),
            Err(AppError::BadRequest(_))
        ));
    }

    #[test]
    fn validate_list_params_applies_defaults_and_clamps() {
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
    fn validate_list_params_parses_all_filters() {
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
    fn validate_list_params_rejects_invalid_filters() {
        assert!(matches!(
            validate_list_params(SubjectRequestListParams {
                status: Some("bad".to_string()),
                request_type: None,
                user_id: None,
                from: None,
                to: None,
                page: None,
                per_page: None,
            }),
            Err(AppError::BadRequest(_))
        ));

        assert!(matches!(
            validate_list_params(SubjectRequestListParams {
                status: None,
                request_type: Some("bad".to_string()),
                user_id: None,
                from: None,
                to: None,
                page: None,
                per_page: None,
            }),
            Err(AppError::BadRequest(_))
        ));
    }

    #[test]
    fn validate_list_params_rejects_inverted_date_range() {
        assert!(matches!(
            validate_list_params(SubjectRequestListParams {
                status: None,
                request_type: None,
                user_id: None,
                from: Some("2026-02-10".to_string()),
                to: Some("2026-02-01".to_string()),
                page: None,
                per_page: None,
            }),
            Err(AppError::BadRequest(_))
        ));
    }

    #[test]
    fn decision_result_keeps_message() {
        let result = MessageResponse::new("Subject request approved");
        assert_eq!(result.message, "Subject request approved");
    }
}
