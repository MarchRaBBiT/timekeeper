use crate::error::AppError;
use axum::{
    body::Body,
    extract::{Extension, Path, Query, State},
    http::{
        header::{CONTENT_DISPOSITION, CONTENT_TYPE},
        HeaderValue,
    },
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};

use crate::{
    models::{audit_log::AuditLog, user::User},
    repositories::{
        audit_log::{self, AuditLogFilters},
        permissions,
    },
    state::AppState,
    types::{AuditLogId, UserId},
    utils::{
        pii::{mask_ip, mask_pii_json, mask_user_agent},
        time,
    },
};
use std::str::FromStr;

const DEFAULT_PAGE: i64 = 1;
const DEFAULT_PER_PAGE: i64 = 25;
const MAX_PER_PAGE: i64 = 100;
const MAX_PAGE: i64 = 1_000;

#[derive(Debug, Deserialize, Serialize, IntoParams, ToSchema)]
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

#[derive(Debug, Deserialize, Serialize, IntoParams, ToSchema)]
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

#[derive(Debug, Serialize, Deserialize, ToSchema)]
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

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AuditLogListResponse {
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub items: Vec<AuditLogResponse>,
}

fn apply_pii_policy(mut response: AuditLogResponse, mask_pii: bool) -> AuditLogResponse {
    if !mask_pii {
        return response;
    }

    response.metadata = response.metadata.as_ref().map(mask_pii_json);
    response.ip = response.ip.as_deref().map(mask_ip);
    response.user_agent = response.user_agent.as_deref().map(mask_user_agent);
    response
}

pub async fn list_audit_logs(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(q): Query<AuditLogListQuery>,
) -> Result<Response, AppError> {
    ensure_audit_log_access(&state.write_pool, &user).await?;

    let (page, per_page, filters) = validate_list_query(q)?;
    let offset = (page - 1) * per_page;
    let (items, total): (Vec<crate::models::audit_log::AuditLog>, i64) =
        audit_log::list_audit_logs(state.read_pool(), &filters, per_page, offset)
            .await
            .map_err(|e| AppError::InternalServerError(e.into()))?;

    let mask_pii = !user.is_system_admin();
    let mut response = Json(AuditLogListResponse {
        page,
        per_page,
        total,
        items: items
            .into_iter()
            .map(AuditLogResponse::from)
            .map(|response| apply_pii_policy(response, mask_pii))
            .collect::<Vec<_>>(),
    })
    .into_response();
    response.headers_mut().insert(
        "X-PII-Masked",
        HeaderValue::from_static(if mask_pii { "true" } else { "false" }),
    );
    Ok(response)
}

pub async fn get_audit_log_detail(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    ensure_audit_log_access(&state.write_pool, &user).await?;

    let audit_log_id = AuditLogId::from_str(&id)
        .map_err(|_| AppError::BadRequest("Invalid audit log ID".into()))?;

    let log = audit_log::fetch_audit_log(state.read_pool(), audit_log_id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?
        .ok_or_else(|| AppError::NotFound("Not found".into()))?;

    let mask_pii = !user.is_system_admin();
    let mut response =
        Json(apply_pii_policy(AuditLogResponse::from(log), mask_pii)).into_response();
    response.headers_mut().insert(
        "X-PII-Masked",
        HeaderValue::from_static(if mask_pii { "true" } else { "false" }),
    );
    Ok(response)
}

pub async fn export_audit_logs(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(q): Query<AuditLogExportQuery>,
) -> Result<Response, AppError> {
    ensure_audit_log_access(&state.write_pool, &user).await?;

    let filters = validate_export_query(q)?;
    let logs = audit_log::export_audit_logs(
        state.read_pool(),
        &filters,
        state.config.audit_log_export_max_rows,
    )
    .await
    .map_err(|e| AppError::InternalServerError(e.into()))?;

    let mask_pii = !user.is_system_admin();
    let payload: Vec<AuditLogResponse> = logs
        .into_iter()
        .map(AuditLogResponse::from)
        .map(|response| apply_pii_policy(response, mask_pii))
        .collect();
    let body = serde_json::to_vec(&payload).map_err(|e| AppError::InternalServerError(e.into()))?;

    let filename = format!(
        "audit_logs_{}.json",
        time::now_in_timezone(&state.config.time_zone).format("%Y%m%d_%H%M%S")
    );
    let mut response = Response::new(Body::from(body));
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    response.headers_mut().insert(
        CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{}\"", filename))
            .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
    );
    response.headers_mut().insert(
        "X-PII-Masked",
        HeaderValue::from_static(if mask_pii { "true" } else { "false" }),
    );
    Ok(response)
}

async fn ensure_audit_log_access(pool: &sqlx::PgPool, user: &User) -> Result<(), AppError> {
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
        Err(AppError::Forbidden("Forbidden".into()))
    }
}

fn validate_list_query(q: AuditLogListQuery) -> Result<(i64, i64, AuditLogFilters), AppError> {
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

const MAX_EXPORT_DAYS: i64 = 31;

fn validate_export_query(q: AuditLogExportQuery) -> Result<AuditLogFilters, AppError> {
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

struct AuditLogFilterInput {
    from: Option<String>,
    to: Option<String>,
    actor_id: Option<String>,
    actor_type: Option<String>,
    event_type: Option<String>,
    target_type: Option<String>,
    target_id: Option<String>,
    result: Option<String>,
}

fn build_filters(input: AuditLogFilterInput) -> Result<AuditLogFilters, AppError> {
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

fn normalize_filter(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
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
