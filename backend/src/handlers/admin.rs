use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::{
    config::Config,
    models::{
        attendance::{Attendance, AttendanceResponse},
        break_record::BreakRecordResponse,
        user::{CreateUser, UpdateUser, User, UserResponse},
    },
    utils::{password::hash_password, time},
};

pub async fn get_users(
    State((pool, config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<UserResponse>>, (StatusCode, Json<Value>)> {
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }
    // Normalize role to snake_case at read to be resilient to legacy rows
    let users = sqlx::query_as::<_, User>(
        "SELECT id, username, password_hash, full_name, LOWER(role) as role, created_at, updated_at FROM users ORDER BY created_at DESC"
    )
    .fetch_all(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        )
    })?;

    let responses = users.into_iter().map(UserResponse::from).collect();
    Ok(Json(responses))
}

pub async fn create_user(
    State((pool, config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Json(payload): Json<CreateUser>,
) -> Result<Json<UserResponse>, (StatusCode, Json<Value>)> {
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }
    // Check if username already exists
    let existing_user = sqlx::query_as::<_, User>(
        "SELECT id, username, password_hash, full_name, role, created_at, updated_at FROM users WHERE username = ?"
    )
    .bind(&payload.username)
    .fetch_optional(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        )
    })?;

    if existing_user.is_some() {
        return Err((
            StatusCode::CONFLICT,
            Json(json!({"error": "Username already exists"})),
        ));
    }

    let password_hash = hash_password(&payload.password).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to hash password"})),
        )
    })?;

    let user = User::new(
        payload.username,
        password_hash,
        payload.full_name,
        payload.role,
    );

    sqlx::query(
        "INSERT INTO users (id, username, password_hash, full_name, role, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&user.id)
    .bind(&user.username)
    .bind(&user.password_hash)
    .bind(&user.full_name)
    // Store enum as snake_case text to match sqlx mapping
    .bind(match user.role { crate::models::user::UserRole::Employee => "employee", crate::models::user::UserRole::Admin => "admin" })
    .bind(&user.created_at)
    .bind(&user.updated_at)
    .execute(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to create user"})),
        )
    })?;

    let response = UserResponse::from(user);
    Ok(Json(response))
}

pub async fn get_all_attendance(
    State((pool, config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<AttendanceResponse>>, (StatusCode, Json<Value>)> {
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }
    let attendances = sqlx::query_as::<_, Attendance>(
        "SELECT id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours, created_at, updated_at FROM attendance ORDER BY date DESC, user_id"
    )
    .fetch_all(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        )
    })?;

    let mut responses = Vec::new();
    for attendance in attendances {
        let break_records = get_break_records(&pool, &attendance.id).await?;
        let response = AttendanceResponse {
            id: attendance.id,
            user_id: attendance.user_id,
            date: attendance.date,
            clock_in_time: attendance.clock_in_time,
            clock_out_time: attendance.clock_out_time,
            status: attendance.status,
            total_work_hours: attendance.total_work_hours,
            break_records,
        };
        responses.push(response);
    }

    Ok(Json(responses))
}

#[derive(Deserialize)]
pub struct ApprovePayload {
    pub comment: String,
}

pub async fn approve_request(
    State((pool, config)): State<(PgPool, Config)>,
    Extension(user): Extension<crate::models::user::User>,
    Path(request_id): Path<String>,
    Json(body): Json<ApprovePayload>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }
    let approver_id = user.id;
    if body.comment.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"comment is required"})),
        ));
    }
    let comment = body.comment;

    // Try to approve leave request first
    let now_utc = time::now_utc(&config.time_zone);
    let result = sqlx::query(
        "UPDATE leave_requests SET status = 'approved', approved_by = ?, approved_at = ?, decision_comment = ?, updated_at = ? WHERE id = ? AND status = 'pending'"
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

    // Try to approve overtime request
    let now_utc = time::now_utc(&config.time_zone);
    let result = sqlx::query(
        "UPDATE overtime_requests SET status = 'approved', approved_by = ?, approved_at = ?, decision_comment = ?, updated_at = ? WHERE id = ? AND status = 'pending'"
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
    Extension(user): Extension<crate::models::user::User>,
    Path(request_id): Path<String>,
    Json(body): Json<RejectPayload>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }
    let approver_id = user.id;
    if body.comment.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"comment is required"})),
        ));
    }
    let comment = body.comment;

    // Try to reject leave request first
    let now_utc = time::now_utc(&config.time_zone);
    let result = sqlx::query(
        "UPDATE leave_requests SET status = 'rejected', rejected_by = ?, rejected_at = ?, decision_comment = ?, updated_at = ? WHERE id = ? AND status = 'pending'"
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

    // Try to reject overtime request
    let now_utc = time::now_utc(&config.time_zone);
    let result = sqlx::query(
        "UPDATE overtime_requests SET status = 'rejected', rejected_by = ?, rejected_at = ?, decision_comment = ?, updated_at = ? WHERE id = ? AND status = 'pending'"
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

// Admin: list requests with simple filters/pagination
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

    // For brevity: only basic filtering implemented; full filters can be extended later
    let mut conditions = vec![];
    let mut binds: Vec<(usize, String)> = vec![];

    if let Some(uid) = q.user_id.clone() {
        conditions.push("user_id = ?".to_string());
        binds.push((binds.len() + 1, uid));
    }
    if let Some(status) = q.status.clone() {
        conditions.push("status = ?".to_string());
        binds.push((binds.len() + 1, status));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // Leave requests
    let leave_sql = format!(
        "SELECT id, user_id, leave_type, start_date, end_date, reason, status, approved_by, approved_at, rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at FROM leave_requests {} ORDER BY created_at DESC LIMIT ? OFFSET ?",
        where_clause
    );
    let mut leave_query =
        sqlx::query_as::<_, crate::models::leave_request::LeaveRequest>(&leave_sql);
    for (_i, v) in &binds {
        leave_query = leave_query.bind(v);
    }
    leave_query = leave_query.bind(per_page);
    leave_query = leave_query.bind(offset);
    let leave_items = leave_query.fetch_all(&pool).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error":"Database error"})),
        )
    })?;

    // Overtime requests
    let ot_sql = format!(
        "SELECT id, user_id, date, planned_hours, reason, status, approved_by, approved_at, rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at FROM overtime_requests {} ORDER BY created_at DESC LIMIT ? OFFSET ?",
        where_clause
    );
    let mut ot_query =
        sqlx::query_as::<_, crate::models::overtime_request::OvertimeRequest>(&ot_sql);
    for (_i, v) in &binds {
        ot_query = ot_query.bind(v);
    }
    ot_query = ot_query.bind(per_page);
    ot_query = ot_query.bind(offset);
    let ot_items = ot_query.fetch_all(&pool).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error":"Database error"})),
        )
    })?;

    Ok(Json(json!({
        "leave_requests": leave_items.into_iter().map(crate::models::leave_request::LeaveRequestResponse::from).collect::<Vec<_>>(),
        "overtime_requests": ot_items.into_iter().map(crate::models::overtime_request::OvertimeRequestResponse::from).collect::<Vec<_>>(),
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
    if let Some(item) = sqlx::query_as::<_, crate::models::leave_request::LeaveRequest>(
        "SELECT id, user_id, leave_type, start_date, end_date, reason, status, approved_by, approved_at, rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at FROM leave_requests WHERE id = ?"
    )
    .bind(&request_id)
    .fetch_optional(&pool)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"Database error"}))))? {
        return Ok(Json(json!({"kind":"leave","data": crate::models::leave_request::LeaveRequestResponse::from(item)})));
    }
    if let Some(item) = sqlx::query_as::<_, crate::models::overtime_request::OvertimeRequest>(
        "SELECT id, user_id, date, planned_hours, reason, status, approved_by, approved_at, rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at FROM overtime_requests WHERE id = ?"
    )
    .bind(&request_id)
    .fetch_optional(&pool)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"Database error"}))))? {
        return Ok(Json(json!({"kind":"overtime","data": crate::models::overtime_request::OvertimeRequestResponse::from(item)})));
    }
    Err((
        StatusCode::NOT_FOUND,
        Json(json!({"error":"Request not found"})),
    ))
}

// Admin: create/replace attendance for a day (basic version)
#[derive(Deserialize)]
pub struct AdminAttendanceUpsert {
    pub user_id: String,
    pub date: String,          // YYYY-MM-DD
    pub clock_in_time: String, // ISO naive or with Z
    pub clock_out_time: Option<String>,
    pub breaks: Option<Vec<AdminBreakItem>>,
}

#[derive(Deserialize)]
pub struct AdminBreakItem {
    pub break_start_time: String,
    pub break_end_time: Option<String>,
}

pub async fn upsert_attendance(
    State((pool, config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Json(body): Json<AdminAttendanceUpsert>,
) -> Result<Json<AttendanceResponse>, (StatusCode, Json<Value>)> {
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }
    use crate::models::attendance::{Attendance, AttendanceResponse, AttendanceStatus};
    use chrono::{NaiveDate, NaiveDateTime};
    let date = NaiveDate::parse_from_str(&body.date, "%Y-%m-%d").map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"Invalid date"})),
        )
    })?;
    let cin = NaiveDateTime::parse_from_str(&body.clock_in_time, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(&body.clock_in_time, "%Y-%m-%d %H:%M:%S")
        })
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error":"Invalid clock_in_time"})),
            )
        })?;
    let cout = match &body.clock_out_time {
        Some(s) => Some(
            NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
                .map_err(|_| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"error":"Invalid clock_out_time"})),
                    )
                })?,
        ),
        None => None,
    };

    // ensure unique per user/date: delete existing and reinsert (basic upsert)
    let _ = sqlx::query("DELETE FROM attendance WHERE user_id = ? AND date = ?")
        .bind(&body.user_id)
        .bind(date)
        .execute(&pool)
        .await;

    let mut att = crate::models::attendance::Attendance::new(
        body.user_id.clone(),
        date,
        time::now_utc(&config.time_zone),
    );
    att.clock_in_time = Some(cin);
    att.clock_out_time = cout;
    att.calculate_work_hours();

    sqlx::query("INSERT INTO attendance (id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)" )
        .bind(&att.id)
        .bind(&att.user_id)
        .bind(&att.date)
        .bind(&att.clock_in_time)
        .bind(&att.clock_out_time)
          // Store enum as snake_case text to match sqlx mapping
          .bind(match att.status { AttendanceStatus::Present => "present", AttendanceStatus::Absent => "absent", AttendanceStatus::Late => "late", AttendanceStatus::HalfDay => "half_day" })
          .bind(&att.total_work_hours)
        .bind(&att.created_at)
        .bind(&att.updated_at)
        .execute(&pool)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"Failed to upsert attendance"}))))?;

    // insert breaks
    if let Some(bks) = body.breaks {
        for b in bks {
            let bs =
                chrono::NaiveDateTime::parse_from_str(&b.break_start_time, "%Y-%m-%dT%H:%M:%S")
                    .or_else(|_| {
                        chrono::NaiveDateTime::parse_from_str(
                            &b.break_start_time,
                            "%Y-%m-%d %H:%M:%S",
                        )
                    })
                    .map_err(|_| {
                        (
                            StatusCode::BAD_REQUEST,
                            Json(json!({"error":"Invalid break_start_time"})),
                        )
                    })?;
            let be: Option<chrono::NaiveDateTime> = b.break_end_time.as_ref().and_then(|s| {
                chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
                    .ok()
                    .or_else(|| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").ok())
            });
            let now_utc = time::now_utc(&config.time_zone);
            let mut br = crate::models::break_record::BreakRecord::new(att.id.clone(), bs, now_utc);
            if let Some(bev) = be {
                br.break_end_time = Some(bev);
                let d = bev - bs;
                br.duration_minutes = Some(d.num_minutes() as i32);
                br.updated_at = now_utc;
            }
            sqlx::query("INSERT INTO break_records (id, attendance_id, break_start_time, break_end_time, duration_minutes, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
                .bind(&br.id)
                .bind(&br.attendance_id)
                .bind(&br.break_start_time)
                .bind(&br.break_end_time)
                .bind(&br.duration_minutes)
                .bind(&br.created_at)
                .bind(&br.updated_at)
                .execute(&pool)
                .await
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"Failed to insert break"}))))?;
        }
    }

    let breaks = get_break_records(&pool, &att.id).await?;
    Ok(Json(AttendanceResponse {
        id: att.id,
        user_id: att.user_id,
        date: att.date,
        clock_in_time: att.clock_in_time,
        clock_out_time: att.clock_out_time,
        status: att.status,
        total_work_hours: att.total_work_hours,
        break_records: breaks,
    }))
}

// Admin: force end a break
pub async fn force_end_break(
    State((pool, config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Path(break_id): Path<String>,
) -> Result<Json<crate::models::break_record::BreakRecordResponse>, (StatusCode, Json<Value>)> {
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }
    let now_local = time::now_in_timezone(&config.time_zone);
    let now_utc = now_local.with_timezone(&Utc);
    let now = now_local.naive_local();
    let mut rec = sqlx::query_as::<_, crate::models::break_record::BreakRecord>(
        "SELECT id, attendance_id, break_start_time, break_end_time, duration_minutes, created_at, updated_at FROM break_records WHERE id = ?"
    )
    .bind(&break_id)
    .fetch_optional(&pool)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"Database error"}))))?
    .ok_or_else(|| (StatusCode::NOT_FOUND, Json(json!({"error":"Break not found"}))))?;

    if rec.break_end_time.is_some() {
        return Err((
            StatusCode::CONFLICT,
            Json(json!({"error":"Break already ended"})),
        ));
    }
    rec.break_end_time = Some(now);
    let d = now - rec.break_start_time;
    rec.duration_minutes = Some(d.num_minutes() as i32);
    rec.updated_at = now_utc;

    sqlx::query("UPDATE break_records SET break_end_time = ?, duration_minutes = ?, updated_at = ? WHERE id = ?")
        .bind(&rec.break_end_time)
        .bind(&rec.duration_minutes)
        .bind(&rec.updated_at)
        .bind(&rec.id)
        .execute(&pool)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"Failed to update break"}))))?;

    Ok(Json(
        crate::models::break_record::BreakRecordResponse::from(rec),
    ))
}

#[derive(Deserialize)]
pub struct ExportQuery {
    pub username: Option<String>,
    pub from: Option<String>, // YYYY-MM-DD
    pub to: Option<String>,   // YYYY-MM-DD
}

pub async fn export_data(
    State((pool, config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Query(q): Query<ExportQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }
    // Build filtered SQL
    use sqlx::Row;
    let mut conditions: Vec<String> = Vec::new();
    let mut binds: Vec<String> = Vec::new();
    if let Some(u) = q.username.as_ref() {
        conditions.push("u.username = ?".to_string());
        binds.push(u.clone());
    }
    if let Some(f) = q.from.as_ref() {
        conditions.push("a.date >= ?".to_string());
        binds.push(f.clone());
    }
    if let Some(t) = q.to.as_ref() {
        conditions.push("a.date <= ?".to_string());
        binds.push(t.clone());
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {} ", conditions.join(" AND "))
    };
    let sql = format!(
        "SELECT u.username, u.full_name, a.date, a.clock_in_time, a.clock_out_time, a.total_work_hours, a.status \
         FROM attendance a JOIN users u ON a.user_id = u.id{} ORDER BY a.date DESC, u.username",
        where_clause
    );
    let mut query = sqlx::query(&sql);
    for v in &binds {
        query = query.bind(v);
    }
    let data = query.fetch_all(&pool).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        )
    })?;

    // Convert to CSV format
    let mut csv_data = String::new();
    csv_data.push_str("Username,Full Name,Date,Clock In,Clock Out,Total Hours,Status\n");

    for record in data {
        csv_data.push_str(&format!(
            "{},{},{},{},{},{},{}\n",
            record.try_get::<String, _>("username").unwrap_or_default(),
            record.try_get::<String, _>("full_name").unwrap_or_default(),
            record.try_get::<String, _>("date").unwrap_or_default(),
            record
                .try_get::<Option<chrono::NaiveDateTime>, _>("clock_in_time")
                .ok()
                .flatten()
                .map(|t| t.format("%H:%M:%S").to_string())
                .unwrap_or_default(),
            record
                .try_get::<Option<chrono::NaiveDateTime>, _>("clock_out_time")
                .ok()
                .flatten()
                .map(|t| t.format("%H:%M:%S").to_string())
                .unwrap_or_default(),
            record.try_get::<f64, _>("total_work_hours").unwrap_or(0.0),
            record.try_get::<String, _>("status").unwrap_or_default(),
        ));
    }

    Ok(Json(json!({
        "csv_data": csv_data,
        "filename": format!(
            "attendance_export_{}.csv",
            time::now_in_timezone(&config.time_zone).format("%Y%m%d_%H%M%S")
        )
    })))
}

async fn get_break_records(
    pool: &PgPool,
    attendance_id: &str,
) -> Result<Vec<BreakRecordResponse>, (StatusCode, Json<Value>)> {
    let break_records = sqlx::query_as::<_, crate::models::break_record::BreakRecord>(
        "SELECT id, attendance_id, break_start_time, break_end_time, duration_minutes, created_at, updated_at FROM break_records WHERE attendance_id = ? ORDER BY break_start_time"
    )
    .bind(attendance_id)
    .fetch_all(pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        )
    })?;

    Ok(break_records
        .into_iter()
        .map(BreakRecordResponse::from)
        .collect())
}
