use axum::{
    body::Body,
    extract::{Extension, Path, Query, State},
    http::{
        header::{CONTENT_DISPOSITION, CONTENT_TYPE},
        HeaderValue, StatusCode,
    },
    response::Response,
    Json,
};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;
use utoipa::{IntoParams, ToSchema};

use crate::{
    config::Config,
    handlers::audit_log_repo::{self, AuditLogFilters},
    models::{audit_log::AuditLog, user::User},
    utils::time,
};

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
    pub from: Option<String>,
    pub to: Option<String>,
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
            id: log.id,
            occurred_at: log.occurred_at,
            actor_id: log.actor_id,
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

pub async fn list_audit_logs(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Query(q): Query<AuditLogListQuery>,
) -> Result<Json<AuditLogListResponse>, (StatusCode, Json<Value>)> {
    ensure_system_admin(&user)?;

    let (page, per_page, filters) = validate_list_query(q)?;
    let offset = (page - 1) * per_page;
    let (items, total) = audit_log_repo::list_audit_logs(&pool, &filters, per_page, offset)
        .await
        .map_err(|err| {
            tracing::error!(error = %err, "failed to list audit logs");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?;

    Ok(Json(AuditLogListResponse {
        page,
        per_page,
        total,
        items: items.into_iter().map(AuditLogResponse::from).collect(),
    }))
}

pub async fn get_audit_log_detail(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> Result<Json<AuditLogResponse>, (StatusCode, Json<Value>)> {
    ensure_system_admin(&user)?;

    let log = audit_log_repo::fetch_audit_log(&pool, &id)
        .await
        .map_err(|err| {
            tracing::error!(error = %err, "failed to fetch audit log");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(json!({"error": "Not found"}))))?;

    Ok(Json(AuditLogResponse::from(log)))
}

pub async fn export_audit_logs(
    State((pool, config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Query(q): Query<AuditLogExportQuery>,
) -> Result<Response, (StatusCode, Json<Value>)> {
    ensure_system_admin(&user)?;

    let filters = validate_export_query(q)?;
    let logs = audit_log_repo::export_audit_logs(&pool, &filters)
        .await
        .map_err(|err| {
            tracing::error!(error = %err, "failed to export audit logs");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?;

    let payload: Vec<AuditLogResponse> = logs.into_iter().map(AuditLogResponse::from).collect();
    let body = serde_json::to_vec(&payload).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to serialize audit logs"})),
        )
    })?;

    let filename = format!(
        "audit_logs_{}.json",
        time::now_in_timezone(&config.time_zone).format("%Y%m%d_%H%M%S")
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
    Ok(response)
}

fn ensure_system_admin(user: &User) -> Result<(), (StatusCode, Json<Value>)> {
    if user.is_system_admin() {
        Ok(())
    } else {
        Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))))
    }
}

fn validate_list_query(
    q: AuditLogListQuery,
) -> Result<(i64, i64, AuditLogFilters), (StatusCode, Json<Value>)> {
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

fn validate_export_query(
    q: AuditLogExportQuery,
) -> Result<AuditLogFilters, (StatusCode, Json<Value>)> {
    build_filters(AuditLogFilterInput {
        from: q.from,
        to: q.to,
        actor_id: q.actor_id,
        actor_type: q.actor_type,
        event_type: q.event_type,
        target_type: q.target_type,
        target_id: q.target_id,
        result: q.result,
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

fn build_filters(input: AuditLogFilterInput) -> Result<AuditLogFilters, (StatusCode, Json<Value>)> {
    let from = parse_from_datetime(input.from.as_deref()).map_err(bad_request)?;
    let to = parse_to_datetime(input.to.as_deref()).map_err(bad_request)?;

    if let (Some(from), Some(to)) = (from, to) {
        if from > to {
            return Err(bad_request("`from` must be before or equal to `to`"));
        }
    }

    let result = normalize_filter(input.result).map(|value| value.to_ascii_lowercase());
    if let Some(ref value) = result {
        if value != "success" && value != "failure" {
            return Err(bad_request("`result` must be success or failure"));
        }
    }

    Ok(AuditLogFilters {
        from,
        to,
        actor_id: normalize_filter(input.actor_id),
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

fn bad_request(message: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": message })))
}
