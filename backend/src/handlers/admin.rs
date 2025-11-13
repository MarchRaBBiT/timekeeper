use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{Error as SqlxError, PgPool, Postgres, QueryBuilder};

use crate::{
    config::Config,
    models::{
        attendance::{Attendance, AttendanceResponse},
        break_record::BreakRecordResponse,
        holiday::{CreateHolidayPayload, Holiday, HolidayResponse},
        user::{CreateUser, User, UserResponse},
    },
    utils::{csv::append_csv_row, password::hash_password, time},
};

pub async fn get_users(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<UserResponse>>, (StatusCode, Json<Value>)> {
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }
    // Normalize role to snake_case at read to be resilient to legacy rows
    let users = sqlx::query_as::<_, User>(
        "SELECT id, username, password_hash, full_name, LOWER(role) as role, is_system_admin, \
         mfa_secret, mfa_enabled_at, created_at, updated_at FROM users ORDER BY created_at DESC",
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
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Json(payload): Json<CreateUser>,
) -> Result<Json<UserResponse>, (StatusCode, Json<Value>)> {
    if !user.is_system_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }
    // Check if username already exists
    let existing_user = sqlx::query_as::<_, User>(
        "SELECT id, username, password_hash, full_name, role, is_system_admin, mfa_secret, \
         mfa_enabled_at, created_at, updated_at FROM users WHERE username = $1",
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
        payload.is_system_admin,
    );

    sqlx::query(
        "INSERT INTO users (id, username, password_hash, full_name, role, is_system_admin, \
         mfa_secret, mfa_enabled_at, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(&user.id)
    .bind(&user.username)
    .bind(&user.password_hash)
    .bind(&user.full_name)
    // Store enum as snake_case text to match sqlx mapping
    .bind(match user.role {
        crate::models::user::UserRole::Employee => "employee",
        crate::models::user::UserRole::Admin => "admin",
    })
    .bind(&user.is_system_admin)
    .bind(&user.mfa_secret)
    .bind(&user.mfa_enabled_at)
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
    State((pool, _config)): State<(PgPool, Config)>,
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
        "UPDATE leave_requests SET status = 'approved', approved_by = $1, approved_at = $2, decision_comment = $3, updated_at = $4 WHERE id = $5 AND status = 'pending'"
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
        "UPDATE overtime_requests SET status = 'approved', approved_by = $1, approved_at = $2, decision_comment = $3, updated_at = $4 WHERE id = $5 AND status = 'pending'"
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
        "UPDATE leave_requests SET status = 'rejected', rejected_by = $1, rejected_at = $2, decision_comment = $3, updated_at = $4 WHERE id = $5 AND status = 'pending'"
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
        "UPDATE overtime_requests SET status = 'rejected', rejected_by = $1, rejected_at = $2, decision_comment = $3, updated_at = $4 WHERE id = $5 AND status = 'pending'"
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

    let type_filter = q.r#type.as_deref().map(|s| s.to_ascii_lowercase());
    let (include_leave, include_overtime) = match type_filter.as_deref() {
        Some("leave") => (true, false),
        Some("overtime") => (false, true),
        Some("all") => (true, true),
        _ => (true, true),
    };

    // Leave requests
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
            .build_query_as::<crate::models::leave_request::LeaveRequest>()
            .fetch_all(&pool)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error":"Database error"})),
                )
            })?
    } else {
        Vec::new()
    };

    // Overtime requests
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
            .build_query_as::<crate::models::overtime_request::OvertimeRequest>()
            .fetch_all(&pool)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error":"Database error"})),
                )
            })?
    } else {
        Vec::new()
    };

    Ok(Json(json!({
        "leave_requests": leave_items.into_iter().map(crate::models::leave_request::LeaveRequestResponse::from).collect::<Vec<_>>(),
        "overtime_requests": ot_items.into_iter().map(crate::models::overtime_request::OvertimeRequestResponse::from).collect::<Vec<_>>(),
        "page_info": {"page": page, "per_page": per_page}
    })))
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

pub async fn get_request_detail(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Path(request_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }
    if let Some(item) = sqlx::query_as::<_, crate::models::leave_request::LeaveRequest>(
        "SELECT id, user_id, leave_type, start_date, end_date, reason, status, approved_by, approved_at, rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at FROM leave_requests WHERE id = $1"
    )
    .bind(&request_id)
    .fetch_optional(&pool)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"Database error"}))))? {
        return Ok(Json(json!({"kind":"leave","data": crate::models::leave_request::LeaveRequestResponse::from(item)})));
    }
    if let Some(item) = sqlx::query_as::<_, crate::models::overtime_request::OvertimeRequest>(
        "SELECT id, user_id, date, planned_hours, reason, status, approved_by, approved_at, rejected_by, rejected_at, cancelled_at, decision_comment, created_at, updated_at FROM overtime_requests WHERE id = $1"
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

#[derive(Deserialize)]
pub struct ResetMfaPayload {
    pub user_id: String,
}

pub async fn upsert_attendance(
    State((pool, config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Json(body): Json<AdminAttendanceUpsert>,
) -> Result<Json<AttendanceResponse>, (StatusCode, Json<Value>)> {
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }
    use crate::models::attendance::{AttendanceResponse, AttendanceStatus};
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
    let _ = sqlx::query("DELETE FROM attendance WHERE user_id = $1 AND date = $2")
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

    sqlx::query("INSERT INTO attendance (id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)" )
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
            sqlx::query("INSERT INTO break_records (id, attendance_id, break_start_time, break_end_time, duration_minutes, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
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
        "SELECT id, attendance_id, break_start_time, break_end_time, duration_minutes, created_at, updated_at FROM break_records WHERE id = $1"
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

    sqlx::query("UPDATE break_records SET break_end_time = $1, duration_minutes = $2, updated_at = $3 WHERE id = $4")
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
    let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT u.username, u.full_name, a.date, a.clock_in_time, a.clock_out_time, a.total_work_hours, a.status \
         FROM attendance a JOIN users u ON a.user_id = u.id",
    );
    enum ExportFilter<'a> {
        Username(&'a String),
        From(&'a String),
        To(&'a String),
    }
    let mut filters = Vec::new();
    if let Some(ref u_name) = q.username {
        filters.push(ExportFilter::Username(u_name));
    }
    if let Some(ref from) = q.from {
        filters.push(ExportFilter::From(from));
    }
    if let Some(ref to) = q.to {
        filters.push(ExportFilter::To(to));
    }
    if !filters.is_empty() {
        builder.push(" WHERE ");
        for (idx, filter) in filters.into_iter().enumerate() {
            if idx > 0 {
                builder.push(" AND ");
            }
            match filter {
                ExportFilter::Username(value) => builder.push("u.username = ").push_bind(value),
                ExportFilter::From(value) => builder.push("a.date >= ").push_bind(value),
                ExportFilter::To(value) => builder.push("a.date <= ").push_bind(value),
            };
        }
    }
    builder.push(" ORDER BY a.date DESC, u.username");
    let data = builder.build().fetch_all(&pool).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        )
    })?;

    // Convert to CSV format
    let mut csv_data = String::new();
    append_csv_row(
        &mut csv_data,
        &[
            "Username".to_string(),
            "Full Name".to_string(),
            "Date".to_string(),
            "Clock In".to_string(),
            "Clock Out".to_string(),
            "Total Hours".to_string(),
            "Status".to_string(),
        ],
    );

    for record in data {
        let username = record.try_get::<String, _>("username").unwrap_or_default();
        let full_name = record.try_get::<String, _>("full_name").unwrap_or_default();
        let date = record.try_get::<String, _>("date").unwrap_or_default();
        let clock_in = record
            .try_get::<Option<chrono::NaiveDateTime>, _>("clock_in_time")
            .ok()
            .flatten()
            .map(|t| t.format("%H:%M:%S").to_string())
            .unwrap_or_default();
        let clock_out = record
            .try_get::<Option<chrono::NaiveDateTime>, _>("clock_out_time")
            .ok()
            .flatten()
            .map(|t| t.format("%H:%M:%S").to_string())
            .unwrap_or_default();
        let total_hours = record
            .try_get::<f64, _>("total_work_hours")
            .map(|h| format!("{:.2}", h))
            .unwrap_or_else(|_| "0.00".to_string());
        let status = record.try_get::<String, _>("status").unwrap_or_default();

        append_csv_row(
            &mut csv_data,
            &[
                username,
                full_name,
                date,
                clock_in,
                clock_out,
                total_hours,
                status,
            ],
        );
    }

    Ok(Json(json!({
        "csv_data": csv_data,
        "filename": format!(
            "attendance_export_{}.csv",
            time::now_in_timezone(&config.time_zone).format("%Y%m%d_%H%M%S")
        )
    })))
}

pub async fn reset_user_mfa(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(requester): Extension<User>,
    Json(payload): Json<ResetMfaPayload>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if !requester.is_system_admin() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error":"Only system administrators can reset MFA"})),
        ));
    }
    let now = Utc::now();
    let result = sqlx::query(
        "UPDATE users SET mfa_secret = NULL, mfa_enabled_at = NULL, updated_at = $1 WHERE id = $2",
    )
    .bind(&now)
    .bind(&payload.user_id)
    .execute(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error":"Failed to reset MFA"})),
        )
    })?;
    if result.rows_affected() == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error":"User not found"})),
        ));
    }
    sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
        .bind(&payload.user_id)
        .execute(&pool)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error":"Failed to revoke refresh tokens"})),
            )
        })?;
    Ok(Json(json!({
        "message": "MFA reset and refresh tokens revoked",
        "user_id": payload.user_id
    })))
}

pub async fn list_holidays(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<HolidayResponse>>, (StatusCode, Json<Value>)> {
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }

    let holidays = sqlx::query_as::<_, Holiday>(
        "SELECT id, holiday_date, name, description, created_at, updated_at \
         FROM holidays ORDER BY holiday_date",
    )
    .fetch_all(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error":"Database error"})),
        )
    })?;

    Ok(Json(
        holidays
            .into_iter()
            .map(HolidayResponse::from)
            .collect::<Vec<_>>(),
    ))
}

pub async fn create_holiday(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Json(payload): Json<CreateHolidayPayload>,
) -> Result<Json<HolidayResponse>, (StatusCode, Json<Value>)> {
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }

    let CreateHolidayPayload {
        holiday_date,
        name,
        description,
    } = payload;

    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"Holiday name is required"})),
        ));
    }

    let normalized_description = description.as_deref().map(str::trim).and_then(|d| {
        if d.is_empty() {
            None
        } else {
            Some(d.to_string())
        }
    });

    let holiday = Holiday::new(
        holiday_date,
        trimmed_name.to_string(),
        normalized_description,
    );

    let insert_result = sqlx::query(
        "INSERT INTO holidays (id, holiday_date, name, description, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(&holiday.id)
    .bind(&holiday.holiday_date)
    .bind(&holiday.name)
    .bind(&holiday.description)
    .bind(&holiday.created_at)
    .bind(&holiday.updated_at)
    .execute(&pool)
    .await;

    match insert_result {
        Ok(_) => Ok(Json(HolidayResponse::from(holiday))),
        Err(SqlxError::Database(db_err))
            if db_err.constraint() == Some("holidays_holiday_date_key") =>
        {
            Err((
                StatusCode::CONFLICT,
                Json(json!({"error":"Holiday already exists for that date"})),
            ))
        }
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error":"Failed to create holiday"})),
        )),
    }
}

pub async fn delete_holiday(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Path(holiday_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }

    let result = sqlx::query("DELETE FROM holidays WHERE id = $1")
        .bind(&holiday_id)
        .execute(&pool)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error":"Failed to delete holiday"})),
            )
        })?;

    if result.rows_affected() == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error":"Holiday not found"})),
        ));
    }

    Ok(Json(json!({"message":"Holiday deleted","id": holiday_id})))
}

async fn get_break_records(
    pool: &PgPool,
    attendance_id: &str,
) -> Result<Vec<BreakRecordResponse>, (StatusCode, Json<Value>)> {
    let break_records = sqlx::query_as::<_, crate::models::break_record::BreakRecord>(
        "SELECT id, attendance_id, break_start_time, break_end_time, duration_minutes, created_at, updated_at FROM break_records WHERE attendance_id = $1 ORDER BY break_start_time"
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
