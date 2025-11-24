use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{PgPool, Postgres, QueryBuilder};
use utoipa::{IntoParams, ToSchema};

use crate::{
    config::Config,
    handlers::requests_repo,
    models::{
        leave_request::{LeaveRequest, LeaveRequestResponse},
        overtime_request::{OvertimeRequest, OvertimeRequestResponse},
        user::User,
    },
    utils::time,
};

const MAX_DECISION_COMMENT_LENGTH: usize = 500;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct ApprovePayload {
    pub comment: String,
}

pub async fn approve_request(
    State((pool, config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Path(request_id): Path<String>,
    Json(body): Json<ApprovePayload>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }
    validate_decision_comment(&body.comment)?;
    let approver_id = user.id;
    let comment = body.comment;
    let now_utc = time::now_utc(&config.time_zone);

    if requests_repo::approve_leave_request(&pool, &request_id, &approver_id, &comment, now_utc)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?
        > 0
    {
        return Ok(Json(json!({"message": "Leave request approved"})));
    }

    if requests_repo::approve_overtime_request(&pool, &request_id, &approver_id, &comment, now_utc)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?
        > 0
    {
        return Ok(Json(json!({"message": "Overtime request approved"})));
    }

    Err((
        StatusCode::NOT_FOUND,
        Json(json!({"error": "Request not found or already processed"})),
    ))
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct RejectPayload {
    pub comment: String,
}

pub async fn reject_request(
    State((pool, config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Path(request_id): Path<String>,
    Json(body): Json<RejectPayload>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }
    validate_decision_comment(&body.comment)?;
    let approver_id = user.id;
    let comment = body.comment;
    let now_utc = time::now_utc(&config.time_zone);

    if requests_repo::reject_leave_request(&pool, &request_id, &approver_id, &comment, now_utc)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?
        > 0
    {
        return Ok(Json(json!({"message": "Leave request rejected"})));
    }

    if requests_repo::reject_overtime_request(&pool, &request_id, &approver_id, &comment, now_utc)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?
        > 0
    {
        return Ok(Json(json!({"message": "Overtime request rejected"})));
    }

    Err((
        StatusCode::NOT_FOUND,
        Json(json!({"error": "Request not found or already processed"})),
    ))
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
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Query(q): Query<RequestListQuery>,
) -> Result<Json<AdminRequestListResponse>, (StatusCode, Json<Value>)> {
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }
    let (page, per_page, offset) = paginate_requests(&q)?;

    let type_filter = q.r#type.as_deref().map(|s| s.to_ascii_lowercase());
    let (include_leave, include_overtime) = match type_filter.as_deref() {
        Some("leave") => (true, false),
        Some("overtime") => (false, true),
        Some("all") => (true, true),
        _ => (true, true),
    };

    let leave_items = if include_leave {
        let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "SELECT id, user_id, leave_type, start_date, end_date, reason, status, approved_by, approved_at, rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at FROM leave_requests",
        );
        apply_request_filters(&mut builder, &q);
        builder
            .push(" ORDER BY created_at DESC LIMIT ")
            .push_bind(per_page)
            .push(" OFFSET ")
            .push_bind(offset);
        builder
            .build_query_as::<LeaveRequest>()
            .fetch_all(&pool)
            .await
            .map_err(|err| {
                tracing::error!(error = %err, "failed to list leave requests");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error":"Database error"})),
                )
            })?
    } else {
        Vec::new()
    };

    let ot_items = if include_overtime {
        let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "SELECT id, user_id, date, planned_hours, reason, status, approved_by, approved_at, rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at FROM overtime_requests",
        );
        apply_request_filters(&mut builder, &q);
        builder
            .push(" ORDER BY created_at DESC LIMIT ")
            .push_bind(per_page)
            .push(" OFFSET ")
            .push_bind(offset);
        builder
            .build_query_as::<OvertimeRequest>()
            .fetch_all(&pool)
            .await
            .map_err(|err| {
                tracing::error!(error = %err, "failed to list overtime requests");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error":"Database error"})),
                )
            })?
    } else {
        Vec::new()
    };

    Ok(Json(AdminRequestListResponse {
        leave_requests: leave_items
            .into_iter()
            .map(LeaveRequestResponse::from)
            .collect::<Vec<_>>(),
        overtime_requests: ot_items
            .into_iter()
            .map(OvertimeRequestResponse::from)
            .collect::<Vec<_>>(),
        page_info: AdminRequestListPageInfo { page, per_page },
    }))
}

pub fn paginate_requests(
    q: &RequestListQuery,
) -> Result<(i64, i64, i64), (StatusCode, Json<Value>)> {
    let page = q.page.unwrap_or(1).max(1);
    let per_page = q.per_page.unwrap_or(20).clamp(1, 100);
    let offset = page
        .checked_sub(1)
        .and_then(|p| p.checked_mul(per_page))
        .ok_or((
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"page is too large"})),
        ))?;
    Ok((page, per_page, offset))
}

pub(crate) fn validate_decision_comment(comment: &str) -> Result<(), (StatusCode, Json<Value>)> {
    if comment.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"comment is required"})),
        ));
    }

    if comment.chars().count() > MAX_DECISION_COMMENT_LENGTH {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": format!(
                    "comment must be between 1 and {} characters",
                    MAX_DECISION_COMMENT_LENGTH
                )
            })),
        ));
    }

    Ok(())
}

pub async fn get_request_detail(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Path(request_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }
    if let Some(item) = requests_repo::fetch_leave_request(&pool, &request_id)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error":"Database error"})),
            )
        })?
    {
        return Ok(Json(
            json!({"kind":"leave","data": LeaveRequestResponse::from(item)}),
        ));
    }
    if let Some(item) = requests_repo::fetch_overtime_request(&pool, &request_id)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error":"Database error"})),
            )
        })?
    {
        return Ok(Json(
            json!({"kind":"overtime","data": OvertimeRequestResponse::from(item)}),
        ));
    }
    Err((
        StatusCode::NOT_FOUND,
        Json(json!({"error":"Request not found"})),
    ))
}

fn apply_request_filters<'a>(
    builder: &mut QueryBuilder<'a, Postgres>,
    filters: &'a RequestListQuery,
) {
    let mut has_clause = false;
    if let Some(ref uid) = filters.user_id {
        push_clause(builder, &mut has_clause);
        builder.push("user_id = ").push_bind(uid);
    }
    if let Some(ref status) = filters.status {
        push_clause(builder, &mut has_clause);
        builder.push("status = ").push_bind(status);
    }
    if let Some(ref from) = filters.from {
        if let Some(from_dt) = parse_filter_datetime(from, false) {
            push_clause(builder, &mut has_clause);
            builder.push("created_at >= ").push_bind(from_dt);
        }
    }
    if let Some(ref to) = filters.to {
        if let Some(to_dt) = parse_filter_datetime(to, true) {
            push_clause(builder, &mut has_clause);
            builder.push("created_at <= ").push_bind(to_dt);
        }
    }
}

fn push_clause(builder: &mut QueryBuilder<'_, Postgres>, has_clause: &mut bool) {
    if *has_clause {
        builder.push(" AND ");
    } else {
        builder.push(" WHERE ");
        *has_clause = true;
    }
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
