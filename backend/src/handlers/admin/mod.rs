use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{Error as SqlxError, FromRow, PgPool, Postgres, QueryBuilder};
use std::str::FromStr;
use utoipa::{IntoParams, ToSchema};

use crate::{
    config::Config,
    handlers::attendance::recalculate_total_hours,
    models::{
        attendance::{Attendance, AttendanceResponse},
        break_record::BreakRecordResponse,
        holiday::{
            CreateHolidayPayload, CreateWeeklyHolidayPayload, Holiday, HolidayResponse,
            WeeklyHoliday, WeeklyHolidayResponse,
        },
        user::{CreateUser, User, UserResponse},
    },
    utils::{csv::append_csv_row, password::hash_password, time},
};

mod requests;
pub use requests::*;

const DEFAULT_PAGE: i64 = 1;
const DEFAULT_PER_PAGE: i64 = 25;
const MAX_PER_PAGE: i64 = 100;
const MAX_PAGE: i64 = 1_000;

pub async fn get_users(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<UserResponse>>, (StatusCode, Json<Value>)> {
    if !user.is_system_admin() {
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
    if !user.is_system_admin() {
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

fn parse_type_filter(raw: Option<&str>) -> Result<Option<AdminHolidayKind>, &'static str> {
    match raw {
        Some(value) if value.eq_ignore_ascii_case("all") => Ok(None),
        Some(value) => AdminHolidayKind::from_str(value)
            .map(Some)
            .map_err(|_| "`type` must be one of public, weekly, exception, all"),
        None => Ok(None),
    }
}

fn parse_optional_date(raw: Option<&str>) -> Result<Option<NaiveDate>, &'static str> {
    match raw {
        Some(value) => parse_date_value(value)
            .ok_or("`from`/`to` must be a valid date (YYYY-MM-DD or RFC3339)")
            .map(Some),
        None => Ok(None),
    }
}

fn parse_date_value(value: &str) -> Option<NaiveDate> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        return Some(dt.date_naive());
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S") {
        return Some(dt.date());
    }
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

fn apply_holiday_filters(
    builder: &mut QueryBuilder<'_, Postgres>,
    has_clause: &mut bool,
    kind: Option<AdminHolidayKind>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
) {
    if let Some(kind) = kind {
        push_clause(builder, has_clause);
        builder.push("kind = ").push_bind(kind.as_str());
    }
    if let Some(from) = from {
        push_clause(builder, has_clause);
        builder.push("applies_from >= ").push_bind(from);
    }
    if let Some(to) = to {
        push_clause(builder, has_clause);
        builder.push("applies_from <= ").push_bind(to);
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

fn bad_request(message: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": message })))
}

async fn fetch_admin_holidays(
    pool: &PgPool,
    kind: Option<AdminHolidayKind>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    per_page: i64,
    offset: i64,
) -> Result<(Vec<AdminHolidayListItem>, i64), SqlxError> {
    let mut data_builder = QueryBuilder::new(
        r#"
        WITH unioned AS (
            SELECT id,
                   'public'::text AS kind,
                   holiday_date AS applies_from,
                   holiday_date AS applies_to,
                   holiday_date AS date,
                   NULL::smallint AS weekday,
                   NULL::date AS starts_on,
                   NULL::date AS ends_on,
                   name,
                   description,
                   NULL::text AS user_id,
                   description AS reason,
                   NULL::text AS created_by,
                   created_at,
                   NULL::boolean AS is_override
            FROM holidays
            UNION ALL
            SELECT id,
                   'weekly'::text AS kind,
                   enforced_from AS applies_from,
                   enforced_to AS applies_to,
                   NULL::date AS date,
                   weekday,
                   starts_on,
                   ends_on,
                   NULL::text AS name,
                   NULL::text AS description,
                   NULL::text AS user_id,
                   NULL::text AS reason,
                   created_by,
                   created_at,
                   NULL::boolean AS is_override
            FROM weekly_holidays
            UNION ALL
            SELECT id,
                   'exception'::text AS kind,
                   exception_date AS applies_from,
                   exception_date AS applies_to,
                   exception_date AS date,
                   NULL::smallint AS weekday,
                   NULL::date AS starts_on,
                   NULL::date AS ends_on,
                   NULL::text AS name,
                   NULL::text AS description,
                   user_id,
                   reason,
                   created_by,
                   created_at,
                   override AS is_override
            FROM holiday_exceptions
        )
        SELECT id, kind, applies_from, applies_to, date, weekday, starts_on, ends_on,
               name, description, user_id, reason, created_by, created_at, is_override
        FROM unioned
        "#,
    );

    let mut data_has_clause = false;
    apply_holiday_filters(&mut data_builder, &mut data_has_clause, kind, from, to);

    data_builder
        .push(" ORDER BY applies_from DESC, kind ASC, created_at DESC")
        .push(" LIMIT ")
        .push_bind(per_page)
        .push(" OFFSET ")
        .push_bind(offset);

    let mut count_builder = QueryBuilder::new(
        r#"
        SELECT COUNT(*) FROM (
            WITH unioned AS (
                SELECT id,
                       'public'::text AS kind,
                       holiday_date AS applies_from,
                       holiday_date AS applies_to,
                       holiday_date AS date,
                       NULL::smallint AS weekday,
                       NULL::date AS starts_on,
                       NULL::date AS ends_on,
                       name,
                       description,
                       NULL::text AS user_id,
                       description AS reason,
                       NULL::text AS created_by,
                       created_at,
                       NULL::boolean AS is_override
                FROM holidays
            UNION ALL
                SELECT id,
                       'weekly'::text AS kind,
                       enforced_from AS applies_from,
                       enforced_to AS applies_to,
                       NULL::date AS date,
                       weekday,
                       starts_on,
                       ends_on,
                       NULL::text AS name,
                       NULL::text AS description,
                       NULL::text AS user_id,
                       NULL::text AS reason,
                       created_by,
                       created_at,
                       NULL::boolean AS is_override
                FROM weekly_holidays
            UNION ALL
                SELECT id,
                       'exception'::text AS kind,
                       exception_date AS applies_from,
                       exception_date AS applies_to,
                       exception_date AS date,
                       NULL::smallint AS weekday,
                       NULL::date AS starts_on,
                       NULL::date AS ends_on,
                       NULL::text AS name,
                       NULL::text AS description,
                       user_id,
                       reason,
                       created_by,
                       created_at,
                       override AS is_override
                FROM holiday_exceptions
            )
            SELECT 1
            FROM unioned
        "#,
    );

    let mut count_has_clause = false;
    apply_holiday_filters(&mut count_builder, &mut count_has_clause, kind, from, to);
    count_builder.push(") AS counted");

    let rows = data_builder
        .build_query_as::<AdminHolidayRow>()
        .fetch_all(pool)
        .await?;

    let total = count_builder
        .build_query_scalar::<i64>()
        .fetch_one(pool)
        .await?;

    let items = rows
        .into_iter()
        .map(AdminHolidayListItem::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| SqlxError::Protocol("invalid holiday kind".into()))?;

    Ok((items, total))
}

// Admin: create/replace attendance for a day (basic version)
#[derive(Deserialize, ToSchema)]
pub struct AdminAttendanceUpsert {
    pub user_id: String,
    pub date: String,          // YYYY-MM-DD
    pub clock_in_time: String, // ISO naive or with Z
    pub clock_out_time: Option<String>,
    pub breaks: Option<Vec<AdminBreakItem>>,
}

#[derive(Deserialize, ToSchema)]
pub struct AdminBreakItem {
    pub break_start_time: String,
    pub break_end_time: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct ResetMfaPayload {
    pub user_id: String,
}

pub async fn upsert_attendance(
    State((pool, config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Json(body): Json<AdminAttendanceUpsert>,
) -> Result<Json<AttendanceResponse>, (StatusCode, Json<Value>)> {
    if !user.is_system_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }
    use crate::models::attendance::{AttendanceResponse, AttendanceStatus};
    use chrono::{NaiveDate, NaiveDateTime};

    let AdminAttendanceUpsert {
        user_id,
        date,
        clock_in_time,
        clock_out_time,
        breaks,
    } = body;

    let date = NaiveDate::parse_from_str(&date, "%Y-%m-%d").map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"Invalid date"})),
        )
    })?;
    let cin = NaiveDateTime::parse_from_str(&clock_in_time, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(&clock_in_time, "%Y-%m-%d %H:%M:%S"))
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error":"Invalid clock_in_time"})),
            )
        })?;
    let cout = match &clock_out_time {
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
        .bind(&user_id)
        .bind(date)
        .execute(&pool)
        .await;

    let mut att = crate::models::attendance::Attendance::new(
        user_id.clone(),
        date,
        time::now_utc(&config.time_zone),
    );
    att.clock_in_time = Some(cin);
    att.clock_out_time = cout;

    let mut total_break_minutes: i64 = 0;
    let mut pending_breaks: Vec<crate::models::break_record::BreakRecord> = Vec::new();

    if let Some(bks) = breaks {
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
                let d = (bev - bs).num_minutes().max(0);
                br.duration_minutes = Some(d as i32);
                br.updated_at = now_utc;
                total_break_minutes += d;
            }
            pending_breaks.push(br);
        }
    }

    att.calculate_work_hours(total_break_minutes);

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
    for br in pending_breaks {
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
    if !user.is_system_admin() {
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

    if let Some(attendance) = sqlx::query_as::<_, Attendance>(
        "SELECT id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours, created_at, updated_at FROM attendance WHERE id = $1"
    )
    .bind(&rec.attendance_id)
    .fetch_optional(&pool)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"Database error"}))))? {
        if attendance.clock_out_time.is_some() {
            recalculate_total_hours(&pool, attendance, now_utc).await?;
        }
    }

    Ok(Json(
        crate::models::break_record::BreakRecordResponse::from(rec),
    ))
}

#[derive(Deserialize, ToSchema, IntoParams)]
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
    Query(q): Query<AdminHolidayListQuery>,
) -> Result<Json<AdminHolidayListResponse>, (StatusCode, Json<Value>)> {
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }

    let page = q.page.unwrap_or(DEFAULT_PAGE).max(1).min(MAX_PAGE);
    let per_page = q
        .per_page
        .unwrap_or(DEFAULT_PER_PAGE)
        .clamp(1, MAX_PER_PAGE);
    let offset = (page - 1) * per_page;

    let type_filter = parse_type_filter(q.r#type.as_deref()).map_err(|msg| bad_request(msg))?;
    let from = parse_optional_date(q.from.as_deref()).map_err(|msg| bad_request(msg))?;
    let to = parse_optional_date(q.to.as_deref()).map_err(|msg| bad_request(msg))?;

    if let (Some(from), Some(to)) = (from, to) {
        if from > to {
            return Err(bad_request("`from` must be before or equal to `to`"));
        }
    }

    let (items, total) = fetch_admin_holidays(&pool, type_filter, from, to, per_page, offset)
        .await
        .map_err(|err| {
            tracing::error!(error = %err, "failed to list admin holidays");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error":"Database error"})),
            )
        })?;

    Ok(Json(AdminHolidayListResponse {
        page,
        per_page,
        total,
        items,
    }))
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

pub async fn list_weekly_holidays(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<WeeklyHolidayResponse>>, (StatusCode, Json<Value>)> {
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }

    let holidays = sqlx::query_as::<_, WeeklyHoliday>(
        "SELECT id, weekday, starts_on, ends_on, enforced_from, enforced_to, created_by, created_at, updated_at \
         FROM weekly_holidays ORDER BY enforced_from, weekday",
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
            .map(WeeklyHolidayResponse::from)
            .collect(),
    ))
}

pub async fn create_weekly_holiday(
    State((pool, config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Json(payload): Json<CreateWeeklyHolidayPayload>,
) -> Result<Json<WeeklyHolidayResponse>, (StatusCode, Json<Value>)> {
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }

    if payload.weekday > 6 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"Weekday must be between 0 (Mon) and 6 (Sun)"})),
        ));
    }

    if let Some(ends_on) = payload.ends_on {
        if ends_on < payload.starts_on {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({"error":"End date must be on or after the start date"})),
            ));
        }
    }

    let today = time::today_local(&config.time_zone);
    let tomorrow = today + Duration::days(1);
    if !user.is_system_admin() && payload.starts_on < tomorrow {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"Start date must be at least tomorrow"})),
        ));
    }

    let weekly = WeeklyHoliday::new(
        payload.weekday,
        payload.starts_on,
        payload.ends_on,
        user.id.clone(),
    );

    sqlx::query(
        "INSERT INTO weekly_holidays \
            (id, weekday, starts_on, ends_on, enforced_from, enforced_to, created_by, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(&weekly.id)
    .bind(&weekly.weekday)
    .bind(&weekly.starts_on)
    .bind(&weekly.ends_on)
    .bind(&weekly.enforced_from)
    .bind(&weekly.enforced_to)
    .bind(&weekly.created_by)
    .bind(&weekly.created_at)
    .bind(&weekly.updated_at)
    .execute(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error":"Failed to create weekly holiday"})),
        )
    })?;

    Ok(Json(WeeklyHolidayResponse::from(weekly)))
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
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct AdminHolidayListQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    #[serde(rename = "type")]
    pub r#type: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AdminHolidayListResponse {
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub items: Vec<AdminHolidayListItem>,
}

#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdminHolidayKind {
    Public,
    Weekly,
    Exception,
}

impl AdminHolidayKind {
    fn as_str(&self) -> &'static str {
        match self {
            AdminHolidayKind::Public => "public",
            AdminHolidayKind::Weekly => "weekly",
            AdminHolidayKind::Exception => "exception",
        }
    }
}

impl FromStr for AdminHolidayKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "public" => Ok(AdminHolidayKind::Public),
            "weekly" => Ok(AdminHolidayKind::Weekly),
            "exception" => Ok(AdminHolidayKind::Exception),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AdminHolidayListItem {
    pub id: String,
    pub kind: AdminHolidayKind,
    pub applies_from: NaiveDate,
    pub applies_to: Option<NaiveDate>,
    pub date: Option<NaiveDate>,
    pub weekday: Option<i16>,
    pub starts_on: Option<NaiveDate>,
    pub ends_on: Option<NaiveDate>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub user_id: Option<String>,
    pub reason: Option<String>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub is_override: Option<bool>,
}
#[derive(Debug, FromRow)]
struct AdminHolidayRow {
    id: String,
    kind: String,
    applies_from: NaiveDate,
    applies_to: Option<NaiveDate>,
    date: Option<NaiveDate>,
    weekday: Option<i16>,
    starts_on: Option<NaiveDate>,
    ends_on: Option<NaiveDate>,
    name: Option<String>,
    description: Option<String>,
    user_id: Option<String>,
    reason: Option<String>,
    created_by: Option<String>,
    created_at: DateTime<Utc>,
    is_override: Option<bool>,
}

impl TryFrom<AdminHolidayRow> for AdminHolidayListItem {
    type Error = ();

    fn try_from(row: AdminHolidayRow) -> Result<Self, Self::Error> {
        let kind = AdminHolidayKind::from_str(&row.kind)?;
        Ok(Self {
            id: row.id,
            kind,
            applies_from: row.applies_from,
            applies_to: row.applies_to,
            date: row.date,
            weekday: row.weekday,
            starts_on: row.starts_on,
            ends_on: row.ends_on,
            name: row.name,
            description: row.description,
            user_id: row.user_id,
            reason: row.reason,
            created_by: row.created_by,
            created_at: row.created_at,
            is_override: row.is_override,
        })
    }
}
