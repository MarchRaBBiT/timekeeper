use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;

use crate::{
    application::http::forbidden_error,
    error::AppError,
    models::{audit_log::AuditLog, user::User},
    repositories::{
        audit_log::{self, AuditLogFilters},
        permissions,
    },
    types::{AuditLogId, UserId},
    utils::pii::{mask_ip, mask_pii_json, mask_user_agent},
};

const DEFAULT_PAGE: i64 = 1;
const DEFAULT_PER_PAGE: i64 = 25;
const MAX_PER_PAGE: i64 = 100;
const MAX_PAGE: i64 = 1_000;
const MAX_EXPORT_DAYS: i64 = 31;

#[derive(Debug, Deserialize, Serialize)]
pub struct AuditLogListQuery {
    pub from: Option<String>,
    pub to: Option<String>,
    pub actor_id: Option<String>,
    pub actor_type: Option<String>,
    pub event_type: Option<String>,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub result: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AuditLogExportQuery {
    pub from: String,
    pub to: String,
    pub actor_id: Option<String>,
    pub actor_type: Option<String>,
    pub event_type: Option<String>,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub result: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditLogResponse {
    pub id: String,
    pub occurred_at: DateTime<Utc>,
    pub actor_id: Option<String>,
    pub actor_type: String,
    pub event_type: String,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub result: String,
    pub error_code: Option<String>,
    pub metadata: Option<Value>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditLogListResponse {
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub items: Vec<AuditLogResponse>,
}

pub struct AuditLogListResult {
    pub response: AuditLogListResponse,
    pub pii_masked: bool,
}

pub struct AuditLogDetailResult {
    pub response: AuditLogResponse,
    pub pii_masked: bool,
}

pub struct AuditLogExportResult {
    pub body: Vec<u8>,
    pub pii_masked: bool,
    pub content_type: &'static str,
}

impl From<AuditLog> for AuditLogResponse {
    fn from(log: AuditLog) -> Self {
        Self {
            id: log.id.to_string(),
            occurred_at: log.occurred_at,
            actor_id: log.actor_id.map(|id| id.to_string()),
            actor_type: log.actor_type,
            event_type: log.event_type,
            target_type: log.target_type,
            target_id: log.target_id,
            result: log.result,
            error_code: log.error_code,
            metadata: log.metadata.map(|value| value.0),
            ip: log.ip,
            user_agent: log.user_agent,
            request_id: log.request_id,
        }
    }
}

pub async fn list_audit_logs(
    read_pool: &sqlx::PgPool,
    user: &User,
    query: AuditLogListQuery,
) -> Result<AuditLogListResult, AppError> {
    ensure_audit_log_access(read_pool, user).await?;

    let (page, per_page, filters) = validate_list_query(query)?;
    let offset = (page - 1) * per_page;
    let (items, total): (Vec<AuditLog>, i64) =
        audit_log::list_audit_logs(read_pool, &filters, per_page, offset)
            .await
            .map_err(|e| AppError::InternalServerError(e.into()))?;

    let mask_pii = !user.is_system_admin();
    Ok(AuditLogListResult {
        response: AuditLogListResponse {
            page,
            per_page,
            total,
            items: items
                .into_iter()
                .map(AuditLogResponse::from)
                .map(|response| apply_pii_policy(response, mask_pii))
                .collect(),
        },
        pii_masked: mask_pii,
    })
}

pub async fn get_audit_log_detail(
    read_pool: &sqlx::PgPool,
    user: &User,
    id: &str,
) -> Result<AuditLogDetailResult, AppError> {
    ensure_audit_log_access(read_pool, user).await?;

    let audit_log_id = AuditLogId::from_str(id)
        .map_err(|_| AppError::BadRequest("Invalid audit log ID".into()))?;

    let log = audit_log::fetch_audit_log(read_pool, audit_log_id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?
        .ok_or_else(|| AppError::NotFound("Not found".into()))?;

    let mask_pii = !user.is_system_admin();
    Ok(AuditLogDetailResult {
        response: apply_pii_policy(AuditLogResponse::from(log), mask_pii),
        pii_masked: mask_pii,
    })
}

pub async fn export_audit_logs(
    read_pool: &sqlx::PgPool,
    user: &User,
    query: AuditLogExportQuery,
) -> Result<AuditLogExportResult, AppError> {
    ensure_audit_log_access(read_pool, user).await?;

    let filters = validate_export_query(query)?;
    let logs = audit_log::export_audit_logs(read_pool, &filters)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    let mask_pii = !user.is_system_admin();
    let payload: Vec<AuditLogResponse> = logs
        .into_iter()
        .map(AuditLogResponse::from)
        .map(|response| apply_pii_policy(response, mask_pii))
        .collect();
    let body = serde_json::to_vec(&payload).map_err(|e| AppError::InternalServerError(e.into()))?;

    Ok(AuditLogExportResult {
        body,
        pii_masked: mask_pii,
        content_type: "application/json",
    })
}

pub fn apply_pii_policy(mut response: AuditLogResponse, mask_pii: bool) -> AuditLogResponse {
    if !mask_pii {
        return response;
    }

    response.metadata = response.metadata.as_ref().map(mask_pii_json);
    response.ip = response.ip.as_deref().map(mask_ip);
    response.user_agent = response.user_agent.as_deref().map(mask_user_agent);
    response
}

pub async fn ensure_audit_log_access(pool: &sqlx::PgPool, user: &User) -> Result<(), AppError> {
    if user.is_system_admin() {
        return Ok(());
    }

    let user_id = user.id.to_string();
    let allowed = permissions::user_has_permission(pool, &user_id, permissions::AUDIT_LOG_READ)
        .await
        .map_err(|err| {
            tracing::error!(error = %err, "failed to check audit log permission");
            AppError::InternalServerError(err.into())
        })?;

    if allowed {
        Ok(())
    } else {
        Err(forbidden_error("Forbidden"))
    }
}

pub fn validate_list_query(q: AuditLogListQuery) -> Result<(i64, i64, AuditLogFilters), AppError> {
    let page = q.page.unwrap_or(DEFAULT_PAGE).clamp(1, MAX_PAGE);
    let per_page = q
        .per_page
        .unwrap_or(DEFAULT_PER_PAGE)
        .clamp(1, MAX_PER_PAGE);
    let filters = build_filters(AuditLogFilterInput {
        from: q.from,
        to: q.to,
        actor_id: q.actor_id,
        actor_type: q.actor_type,
        event_type: q.event_type,
        target_type: q.target_type,
        target_id: q.target_id,
        result: q.result,
    })?;
    Ok((page, per_page, filters))
}

pub fn validate_export_query(q: AuditLogExportQuery) -> Result<AuditLogFilters, AppError> {
    let from = parse_datetime_value(&q.from, true).ok_or_else(|| {
        AppError::BadRequest("`from` must be a valid datetime (RFC3339 or YYYY-MM-DD)".into())
    })?;
    let to = parse_datetime_value(&q.to, false).ok_or_else(|| {
        AppError::BadRequest("`to` must be a valid datetime (RFC3339 or YYYY-MM-DD)".into())
    })?;

    if from > to {
        return Err(AppError::BadRequest(
            "`from` must be before or equal to `to`".into(),
        ));
    }

    let calendar_days = (to.date_naive() - from.date_naive()).num_days();
    if calendar_days > MAX_EXPORT_DAYS {
        return Err(AppError::BadRequest(format!(
            "エクスポート期間は最大{}日です",
            MAX_EXPORT_DAYS
        )));
    }

    let result = normalize_filter(q.result).map(|value| value.to_ascii_lowercase());
    if let Some(ref value) = result {
        if value != "success" && value != "failure" {
            return Err(AppError::BadRequest(
                "`result` must be success or failure".into(),
            ));
        }
    }

    let actor_id = q
        .actor_id
        .filter(|s| !s.trim().is_empty())
        .map(|s| UserId::from_str(&s))
        .transpose()
        .map_err(|_| AppError::BadRequest("Invalid actor ID".into()))?;

    Ok(AuditLogFilters {
        from: Some(from),
        to: Some(to),
        actor_id,
        actor_type: normalize_filter(q.actor_type).map(|value| value.to_ascii_lowercase()),
        event_type: normalize_filter(q.event_type).map(|value| value.to_ascii_lowercase()),
        target_type: normalize_filter(q.target_type).map(|value| value.to_ascii_lowercase()),
        target_id: normalize_filter(q.target_id),
        result,
    })
}

pub struct AuditLogFilterInput {
    pub from: Option<String>,
    pub to: Option<String>,
    pub actor_id: Option<String>,
    pub actor_type: Option<String>,
    pub event_type: Option<String>,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub result: Option<String>,
}

pub fn build_filters(input: AuditLogFilterInput) -> Result<AuditLogFilters, AppError> {
    let from =
        parse_from_datetime(input.from.as_deref()).map_err(|e| AppError::BadRequest(e.into()))?;
    let to = parse_to_datetime(input.to.as_deref()).map_err(|e| AppError::BadRequest(e.into()))?;

    if let (Some(from), Some(to)) = (from, to) {
        if from > to {
            return Err(AppError::BadRequest(
                "`from` must be before or equal to `to`".into(),
            ));
        }
    }

    let result = normalize_filter(input.result).map(|value| value.to_ascii_lowercase());
    if let Some(ref value) = result {
        if value != "success" && value != "failure" {
            return Err(AppError::BadRequest(
                "`result` must be success or failure".into(),
            ));
        }
    }

    let actor_id = input
        .actor_id
        .filter(|s| !s.trim().is_empty())
        .map(|s| UserId::from_str(&s))
        .transpose()
        .map_err(|_| AppError::BadRequest("Invalid actor ID".into()))?;

    Ok(AuditLogFilters {
        from,
        to,
        actor_id,
        actor_type: normalize_filter(input.actor_type).map(|value| value.to_ascii_lowercase()),
        event_type: normalize_filter(input.event_type).map(|value| value.to_ascii_lowercase()),
        target_type: normalize_filter(input.target_type).map(|value| value.to_ascii_lowercase()),
        target_id: normalize_filter(input.target_id),
        result,
    })
}

pub fn normalize_filter(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn parse_from_datetime(raw: Option<&str>) -> Result<Option<DateTime<Utc>>, &'static str> {
    match raw {
        Some(value) => parse_datetime_value(value, true)
            .ok_or("`from` must be a valid datetime (RFC3339 or YYYY-MM-DD)")
            .map(Some),
        None => Ok(None),
    }
}

pub fn parse_to_datetime(raw: Option<&str>) -> Result<Option<DateTime<Utc>>, &'static str> {
    match raw {
        Some(value) => parse_datetime_value(value, false)
            .ok_or("`to` must be a valid datetime (RFC3339 or YYYY-MM-DD)")
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
    use crate::models::user::{User, UserRole};

    fn sample_user(is_system_admin: bool) -> User {
        let now = Utc::now();
        User {
            id: UserId::new(),
            username: "admin".to_string(),
            password_hash: "hash".to_string(),
            full_name: "Admin".to_string(),
            email: "admin@example.com".to_string(),
            role: UserRole::Admin,
            is_system_admin,
            mfa_secret: None,
            mfa_enabled_at: None,
            password_changed_at: now,
            failed_login_attempts: 0,
            locked_until: None,
            lock_reason: None,
            lockout_count: 0,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn apply_pii_policy_masks_sensitive_fields() {
        let response = AuditLogResponse {
            id: "1".to_string(),
            occurred_at: Utc::now(),
            actor_id: Some("user-1".to_string()),
            actor_type: "user".to_string(),
            event_type: "login".to_string(),
            target_type: None,
            target_id: None,
            result: "success".to_string(),
            error_code: None,
            metadata: Some(serde_json::json!({"email":"alice@example.com"})),
            ip: Some("203.0.113.10".to_string()),
            user_agent: Some("Mozilla/5.0".to_string()),
            request_id: None,
        };

        let masked = apply_pii_policy(response, true);
        assert!(masked.metadata.is_some());
        assert_ne!(masked.ip.as_deref(), Some("203.0.113.10"));
        assert_ne!(masked.user_agent.as_deref(), Some("Mozilla/5.0"));
    }

    #[test]
    fn validate_list_query_applies_defaults() {
        let (page, per_page, filters) = validate_list_query(AuditLogListQuery {
            from: None,
            to: None,
            actor_id: None,
            actor_type: None,
            event_type: None,
            target_type: None,
            target_id: None,
            result: None,
            page: None,
            per_page: None,
        })
        .expect("query should validate");

        assert_eq!(page, DEFAULT_PAGE);
        assert_eq!(per_page, DEFAULT_PER_PAGE);
        assert!(filters.from.is_none());
    }

    #[test]
    fn validate_export_query_rejects_long_range() {
        let result = validate_export_query(AuditLogExportQuery {
            from: "2026-01-01".to_string(),
            to: "2026-03-01".to_string(),
            actor_id: None,
            actor_type: None,
            event_type: None,
            target_type: None,
            target_id: None,
            result: None,
        });

        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[test]
    fn parse_datetime_value_supports_date_only_end_of_day() {
        let dt = parse_datetime_value("2026-03-09", false).expect("date should parse");
        assert_eq!(
            dt.time(),
            NaiveTime::from_hms_opt(23, 59, 59).expect("valid time")
        );
    }

    #[test]
    fn normalize_filter_trims_blank_values() {
        assert_eq!(
            normalize_filter(Some("  success ".to_string())),
            Some("success".to_string())
        );
        assert_eq!(normalize_filter(Some("   ".to_string())), None);
    }

    #[test]
    fn build_filters_rejects_invalid_actor_id() {
        let result = build_filters(AuditLogFilterInput {
            from: None,
            to: None,
            actor_id: Some("bad-id".to_string()),
            actor_type: None,
            event_type: None,
            target_type: None,
            target_id: None,
            result: None,
        });
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[test]
    fn system_admin_user_is_not_marked_for_masking() {
        let user = sample_user(true);
        assert!(user.is_system_admin());
    }
}
