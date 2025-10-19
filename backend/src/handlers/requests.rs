use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::{
    config::Config,
    models::{
        leave_request::{CreateLeaveRequest, LeaveRequest, LeaveRequestResponse},
        overtime_request::{CreateOvertimeRequest, OvertimeRequest, OvertimeRequestResponse},
    },
};

use chrono::Utc;
use serde::Deserialize;

pub async fn create_leave_request(
    State((pool, config)): State<(PgPool, Config)>,
    Extension(user): Extension<crate::models::user::User>,
    Json(payload): Json<CreateLeaveRequest>,
) -> Result<Json<LeaveRequestResponse>, (StatusCode, Json<Value>)> {
    let user_id = user.id;

    let leave_request = LeaveRequest::new(
        user_id.to_string(),
        payload.leave_type,
        payload.start_date,
        payload.end_date,
        payload.reason,
    );

    sqlx::query(
        "INSERT INTO leave_requests (id, user_id, leave_type, start_date, end_date, reason, status, approved_by, approved_at, decision_comment, rejected_by, rejected_at, cancelled_at, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&leave_request.id)
    .bind(&leave_request.user_id)
    .bind(match leave_request.leave_type {
        crate::models::leave_request::LeaveType::Annual => "annual",
        crate::models::leave_request::LeaveType::Sick => "sick",
        crate::models::leave_request::LeaveType::Personal => "personal",
        crate::models::leave_request::LeaveType::Other => "other",
    })
    .bind(&leave_request.start_date)
    .bind(&leave_request.end_date)
    .bind(&leave_request.reason)
    .bind(match leave_request.status {
        crate::models::leave_request::RequestStatus::Pending => "pending",
        crate::models::leave_request::RequestStatus::Approved => "approved",
        crate::models::leave_request::RequestStatus::Rejected => "rejected",
        crate::models::leave_request::RequestStatus::Cancelled => "cancelled",
    })
    .bind(&leave_request.approved_by)
    .bind(&leave_request.approved_at)
    .bind(&leave_request.decision_comment)
    .bind(&leave_request.rejected_by)
    .bind(&leave_request.rejected_at)
    .bind(&leave_request.cancelled_at)
    .bind(&leave_request.created_at)
    .bind(&leave_request.updated_at)
    .execute(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to create leave request"})),
        )
    })?;

    let response = LeaveRequestResponse::from(leave_request);
    Ok(Json(response))
}

pub async fn create_overtime_request(
    State((pool, config)): State<(PgPool, Config)>,
    Extension(user): Extension<crate::models::user::User>,
    Json(payload): Json<CreateOvertimeRequest>,
) -> Result<Json<OvertimeRequestResponse>, (StatusCode, Json<Value>)> {
    let user_id = user.id;

    let overtime_request = OvertimeRequest::new(
        user_id.to_string(),
        payload.date,
        payload.planned_hours,
        payload.reason,
    );

    sqlx::query(
        "INSERT INTO overtime_requests (id, user_id, date, planned_hours, reason, status, approved_by, approved_at, decision_comment, rejected_by, rejected_at, cancelled_at, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&overtime_request.id)
    .bind(&overtime_request.user_id)
    .bind(&overtime_request.date)
    .bind(&overtime_request.planned_hours)
    .bind(&overtime_request.reason)
    .bind(match overtime_request.status {
        crate::models::overtime_request::RequestStatus::Pending => "pending",
        crate::models::overtime_request::RequestStatus::Approved => "approved",
        crate::models::overtime_request::RequestStatus::Rejected => "rejected",
        crate::models::overtime_request::RequestStatus::Cancelled => "cancelled",
    })
    .bind(&overtime_request.approved_by)
    .bind(&overtime_request.approved_at)
    .bind(&overtime_request.decision_comment)
    .bind(&overtime_request.rejected_by)
    .bind(&overtime_request.rejected_at)
    .bind(&overtime_request.cancelled_at)
    .bind(&overtime_request.created_at)
    .bind(&overtime_request.updated_at)
    .execute(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to create overtime request"})),
        )
    })?;

    let response = OvertimeRequestResponse::from(overtime_request);
    Ok(Json(response))
}

pub async fn get_my_requests(
    State((pool, config)): State<(PgPool, Config)>,
    Extension(user): Extension<crate::models::user::User>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<Value>)> {
    let user_id = user.id;

    // Get leave requests
    let leave_requests = sqlx::query_as::<_, LeaveRequest>(
        "SELECT id, user_id, leave_type, start_date, end_date, reason, status, approved_by, approved_at, rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at FROM leave_requests WHERE user_id = ? ORDER BY created_at DESC"
    )
    .bind(&user_id)
    .fetch_all(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        )
    })?;

    // Get overtime requests
    let overtime_requests = sqlx::query_as::<_, OvertimeRequest>(
        "SELECT id, user_id, date, planned_hours, reason, status, approved_by, approved_at, rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at FROM overtime_requests WHERE user_id = ? ORDER BY created_at DESC"
    )
    .bind(&user_id)
    .fetch_all(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        )
    })?;

    let response = json!({
        "leave_requests": leave_requests.into_iter().map(LeaveRequestResponse::from).collect::<Vec<_>>(),
        "overtime_requests": overtime_requests.into_iter().map(OvertimeRequestResponse::from).collect::<Vec<_>>()
    });

    Ok(Json(response))
}

#[derive(Deserialize)]
pub struct UpdateLeavePayload {
    pub leave_type: Option<crate::models::leave_request::LeaveType>,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub reason: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateOvertimePayload {
    pub date: Option<chrono::NaiveDate>,
    pub planned_hours: Option<f64>,
    pub reason: Option<String>,
}

pub async fn update_request(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<crate::models::user::User>,
    Path(request_id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let user_id = user.id;

    // Try leave update
    if let Some(mut req) = sqlx::query_as::<_, LeaveRequest>(
        "SELECT id, user_id, leave_type, start_date, end_date, reason, status, approved_by, approved_at, rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at FROM leave_requests WHERE id = ? AND user_id = ?"
    )
    .bind(&request_id)
    .bind(&user_id)
    .fetch_optional(&pool)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"Database error"}))))? {
        if !matches!(req.status, crate::models::leave_request::RequestStatus::Pending) {
            return Err((StatusCode::CONFLICT, Json(json!({"error":"Only pending requests can be updated"}))));
        }
        let upd: UpdateLeavePayload = serde_json::from_value(payload.clone()).map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error":"Invalid payload"}))))?;
        let new_type = upd.leave_type.unwrap_or(req.leave_type);
        let new_start = upd.start_date.unwrap_or(req.start_date);
        let new_end = upd.end_date.unwrap_or(req.end_date);
        if new_start > new_end { return Err((StatusCode::BAD_REQUEST, Json(json!({"error":"start_date must be <= end_date"})))); }
        let new_reason = upd.reason.or(req.reason);
        let now = Utc::now();
        sqlx::query("UPDATE leave_requests SET leave_type = ?, start_date = ?, end_date = ?, reason = ?, updated_at = ? WHERE id = ?")
            .bind(match new_type {
                crate::models::leave_request::LeaveType::Annual => "annual",
                crate::models::leave_request::LeaveType::Sick => "sick",
                crate::models::leave_request::LeaveType::Personal => "personal",
                crate::models::leave_request::LeaveType::Other => "other",
            })
            .bind(&new_start)
            .bind(&new_end)
            .bind(&new_reason)
            .bind(&now)
            .bind(&req.id)
            .execute(&pool)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"Failed to update request"}))))?;
        return Ok(Json(json!({"message":"Leave request updated"})));
    }

    // Try overtime update
    if let Some(mut req) = sqlx::query_as::<_, OvertimeRequest>(
        "SELECT id, user_id, date, planned_hours, reason, status, approved_by, approved_at, rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at FROM overtime_requests WHERE id = ? AND user_id = ?"
    )
    .bind(&request_id)
    .bind(&user_id)
    .fetch_optional(&pool)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"Database error"}))))? {
        if !matches!(req.status, crate::models::overtime_request::RequestStatus::Pending) {
            return Err((StatusCode::CONFLICT, Json(json!({"error":"Only pending requests can be updated"}))));
        }
        let upd: UpdateOvertimePayload = serde_json::from_value(payload.clone()).map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error":"Invalid payload"}))))?;
        let new_date = upd.date.unwrap_or(req.date);
        let new_hours = upd.planned_hours.unwrap_or(req.planned_hours);
        if new_hours <= 0.0 { return Err((StatusCode::BAD_REQUEST, Json(json!({"error":"planned_hours must be > 0"})))); }
        let new_reason = upd.reason.or(req.reason);
        let now = Utc::now();
        sqlx::query("UPDATE overtime_requests SET date = ?, planned_hours = ?, reason = ?, updated_at = ? WHERE id = ?")
            .bind(&new_date)
            .bind(&new_hours)
            .bind(&new_reason)
            .bind(&now)
            .bind(&req.id)
            .execute(&pool)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"Failed to update request"}))))?;
        return Ok(Json(json!({"message":"Overtime request updated"})));
    }

    Err((
        StatusCode::NOT_FOUND,
        Json(json!({"error":"Request not found"})),
    ))
}

pub async fn cancel_request(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<crate::models::user::User>,
    Path(request_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let user_id = user.id;
    // Leave
    let result = sqlx::query(
        "UPDATE leave_requests SET status = 'cancelled', cancelled_at = ?, updated_at = ? WHERE id = ? AND user_id = ? AND status = 'pending'"
    )
    .bind(&Utc::now())
    .bind(&Utc::now())
    .bind(&request_id)
    .bind(&user_id)
    .execute(&pool)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"Database error"}))))?;
    if result.rows_affected() > 0 {
        return Ok(Json(json!({"id": request_id, "status":"cancelled"})));
    }

    // Overtime
    let result = sqlx::query(
        "UPDATE overtime_requests SET status = 'cancelled', cancelled_at = ?, updated_at = ? WHERE id = ? AND user_id = ? AND status = 'pending'"
    )
    .bind(&Utc::now())
    .bind(&Utc::now())
    .bind(&request_id)
    .bind(&user_id)
    .execute(&pool)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"Database error"}))))?;
    if result.rows_affected() > 0 {
        return Ok(Json(json!({"id": request_id, "status":"cancelled"})));
    }

    Err((
        StatusCode::NOT_FOUND,
        Json(json!({"error":"Request not found or not cancellable"})),
    ))
}
