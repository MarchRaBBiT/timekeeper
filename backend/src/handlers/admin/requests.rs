use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{PgPool, Postgres, QueryBuilder};

use crate::{
    config::Config,
    models::{
        leave_request::{LeaveRequest, LeaveRequestResponse},
        overtime_request::{OvertimeRequest, OvertimeRequestResponse},
        user::User,
    },
    utils::time,
};

#[derive(Deserialize)]
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
    if body.comment.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"comment is required"})),
        ));
    }
    let approver_id = user.id;
    let comment = body.comment;
    let now_utc = time::now_utc(&config.time_zone);

    let result = sqlx::query(
        "UPDATE leave_requests SET status = 'approved', approved_by = $1, approved_at = $2, decision_comment = $3, updated_at = $4 WHERE id = $5 AND status = 'pending'",
    )
    .bind(&approver_id)
    .bind(&now_utc)
    .bind(&comment)
    .bind(&now_utc)
    .bind(&request_id)
    .execute(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        )
    })?;
    if result.rows_affected() > 0 {
        return Ok(Json(json!({"message": "Leave request approved"})));
    }

    let now_utc = time::now_utc(&config.time_zone);
    let result = sqlx::query(
        "UPDATE overtime_requests SET status = 'approved', approved_by = $1, approved_at = $2, decision_comment = $3, updated_at = $4 WHERE id = $5 AND status = 'pending'",
    )
    .bind(&approver_id)
    .bind(&now_utc)
    .bind(&comment)
    .bind(&now_utc)
    .bind(&request_id)
    .execute(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        )
    })?;
    if result.rows_affected() > 0 {
        return Ok(Json(json!({"message": "Overtime request approved"})));
    }

    Err((
        StatusCode::NOT_FOUND,
        Json(json!({"error": "Request not found or already processed"})),
    ))
}

#[derive(Deserialize)]
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
    if body.comment.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"comment is required"})),
        ));
    }
    let approver_id = user.id;
    let comment = body.comment;
    let now_utc = time::now_utc(&config.time_zone);

    let result = sqlx::query(
        "UPDATE leave_requests SET status = 'rejected', rejected_by = $1, rejected_at = $2, decision_comment = $3, updated_at = $4 WHERE id = $5 AND status = 'pending'",
    )
    .bind(&approver_id)
    .bind(&now_utc)
    .bind(&comment)
    .bind(&now_utc)
    .bind(&request_id)
    .execute(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        )
    })?;
    if result.rows_affected() > 0 {
        return Ok(Json(json!({"message": "Leave request rejected"})));
    }

    let now_utc = time::now_utc(&config.time_zone);
    let result = sqlx::query(
        "UPDATE overtime_requests SET status = 'rejected', rejected_by = $1, rejected_at = $2, decision_comment = $3, updated_at = $4 WHERE id = $5 AND status = 'pending'",
    )
    .bind(&approver_id)
    .bind(&now_utc)
    .bind(&comment)
    .bind(&now_utc)
    .bind(&request_id)
    .execute(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        )
    })?;
    if result.rows_affected() > 0 {
        return Ok(Json(json!({"message": "Overtime request rejected"})));
    }

    Err((
        StatusCode::NOT_FOUND,
        Json(json!({"error": "Request not found or already processed"})),
    ))
}

#[derive(Deserialize)]
pub struct RequestListQuery {
    pub status: Option<String>,
    pub r#type: Option<String>,
    pub user_id: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

pub async fn list_requests(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Query(q): Query<RequestListQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }
    let page = q.page.unwrap_or(1).max(1);
    let per_page = q.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;

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

    Ok(Json(json!({
        "leave_requests": leave_items.into_iter().map(LeaveRequestResponse::from).collect::<Vec<_>>(),
        "overtime_requests": ot_items.into_iter().map(OvertimeRequestResponse::from).collect::<Vec<_>>(),
        "page_info": {"page": page, "per_page": per_page}
    })))
}

pub async fn get_request_detail(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Path(request_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }
    if let Some(item) = sqlx::query_as::<_, LeaveRequest>(
        "SELECT id, user_id, leave_type, start_date, end_date, reason, status, approved_by, approved_at, rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at FROM leave_requests WHERE id = $1",
    )
    .bind(&request_id)
    .fetch_optional(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error":"Database error"})),
        )
    })? {
        return Ok(Json(json!({"kind":"leave","data": LeaveRequestResponse::from(item)})));
    }
    if let Some(item) = sqlx::query_as::<_, OvertimeRequest>(
        "SELECT id, user_id, date, planned_hours, reason, status, approved_by, approved_at, rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at FROM overtime_requests WHERE id = $1",
    )
    .bind(&request_id)
    .fetch_optional(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error":"Database error"})),
        )
    })? {
        return Ok(Json(json!({"kind":"overtime","data": OvertimeRequestResponse::from(item)})));
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
