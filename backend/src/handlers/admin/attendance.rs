use axum::{
    extract::{Extension, Path, State},
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use sqlx::{PgPool, Transaction, Postgres};
use std::str::FromStr;
use utoipa::ToSchema;
use crate::error::AppError;
use crate::types::{AttendanceId, UserId, BreakRecordId};

use crate::{
    config::Config,
    handlers::attendance::recalculate_total_hours,
    models::{
        attendance::{Attendance, AttendanceResponse, AttendanceStatus},
        break_record::BreakRecordResponse,
        user::User,
    },
    utils::time,
};

pub async fn get_all_attendance(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<AttendanceResponse>>, AppError> {
    if !user.is_system_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }
    let attendances = sqlx::query_as::<_, Attendance>(
        "SELECT id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours, created_at, updated_at FROM attendance ORDER BY date DESC, user_id"
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::InternalServerError(e.into()))?;

    let mut responses = Vec::new();
    for attendance in attendances {
        let break_records = get_break_records(&pool, attendance.id).await?;
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

pub async fn upsert_attendance(
    State((pool, config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Json(body): Json<AdminAttendanceUpsert>,
) -> Result<Json<AttendanceResponse>, AppError> {
    if !user.is_system_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }
    use crate::models::attendance::AttendanceResponse;
    use chrono::{NaiveDate, NaiveDateTime};

    let AdminAttendanceUpsert {
        user_id,
        date,
        clock_in_time,
        clock_out_time,
        breaks,
    } = body;

    let date = NaiveDate::parse_from_str(&date, "%Y-%m-%d").map_err(|_| {
        AppError::BadRequest("Invalid date".into())
    })?;
    let cin = NaiveDateTime::parse_from_str(&clock_in_time, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(&clock_in_time, "%Y-%m-%d %H:%M:%S"))
        .map_err(|_| {
            AppError::BadRequest("Invalid clock_in_time".into())
        })?;
    let cout = match &clock_out_time {
        Some(s) => Some(
            NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
                .map_err(|_| {
                    AppError::BadRequest("Invalid clock_out_time".into())
                })?,
        ),
        None => None,
    };

    let mut tx: Transaction<'_, Postgres> = pool.begin().await.map_err(|e| AppError::InternalServerError(e.into()))?;

    // Parse and validate user_id
    let user_id_typed = UserId::from_str(&user_id)
        .map_err(|_| AppError::BadRequest("Invalid user_id format".into()))?;

    // ensure unique per user/date: delete existing and reinsert (basic upsert)
    sqlx::query::<sqlx::Postgres>("DELETE FROM attendance WHERE user_id = $1 AND date = $2")
        .bind(user_id_typed.to_string())
        .bind(date)
        .execute(&mut *tx)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServerError(e.into()))?;

    let mut att = crate::models::attendance::Attendance::new(
        user_id_typed,
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
                        AppError::BadRequest("Invalid break_start_time".into())
                    })?;
            let be: Option<chrono::NaiveDateTime> = b.break_end_time.as_ref().and_then(|s| {
                chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
                    .ok()
                    .or_else(|| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").ok())
            });
            let now_utc = time::now_utc(&config.time_zone);
            let mut br = crate::models::break_record::BreakRecord::new(att.id, bs, now_utc);
            if let Some(bev) = be {
                br.break_end_time = Some(bev);
                let duration = bev.signed_duration_since(bs);
                let d = duration.num_minutes().max(0);
                br.duration_minutes = Some(d as i32);
                br.updated_at = now_utc;
                total_break_minutes += d;
            }
            pending_breaks.push(br);
        }
    }

    att.calculate_work_hours(total_break_minutes);

    sqlx::query::<sqlx::Postgres>("INSERT INTO attendance (id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)" )
        .bind(att.id.to_string())
        .bind(att.user_id.to_string())
        .bind(att.date)
        .bind(att.clock_in_time)
        .bind(att.clock_out_time)
          // Store enum as snake_case text to match sqlx mapping
          .bind(match att.status { AttendanceStatus::Present => "present", AttendanceStatus::Absent => "absent", AttendanceStatus::Late => "late", AttendanceStatus::HalfDay => "half_day" })
          .bind(att.total_work_hours)
        .bind(att.created_at)
        .bind(att.updated_at)
        .execute(&mut *tx)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServerError(e.into()))?;

    // insert breaks
    for br in pending_breaks {
        sqlx::query::<sqlx::Postgres>("INSERT INTO break_records (id, attendance_id, break_start_time, break_end_time, duration_minutes, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(br.id.to_string())
            .bind(br.attendance_id.to_string())
            .bind(br.break_start_time)
            .bind(br.break_end_time)
            .bind(br.duration_minutes)
            .bind(br.created_at)
            .bind(br.updated_at)
            .execute(&mut *tx)
            .await
            .map_err(|e: sqlx::Error| AppError::InternalServerError(e.into()))?;
    }

    tx.commit().await.map_err(|e: sqlx::Error| AppError::InternalServerError(e.into()))?;

    let breaks = get_break_records(&pool, att.id).await?;
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
// Admin: force end a break
pub async fn force_end_break(
    State((pool, config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Path(break_id): Path<String>,
) -> Result<Json<crate::models::break_record::BreakRecordResponse>, AppError> {
    if !user.is_system_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }
    let break_record_id = BreakRecordId::from_str(&break_id)
        .map_err(|_| AppError::BadRequest("Invalid break record ID format".into()))?;
    let now_local = time::now_in_timezone(&config.time_zone);
    let now_utc = now_local.with_timezone(&Utc);
    let now = now_local.naive_local();
    let mut rec = sqlx::query_as::<_, crate::models::break_record::BreakRecord>(
        "SELECT id, attendance_id, break_start_time, break_end_time, duration_minutes, created_at, updated_at FROM break_records WHERE id = $1"
    )
    .bind(break_record_id.to_string())
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::InternalServerError(e.into()))?
    .ok_or_else(|| AppError::NotFound("Break not found".into()))?;

    if rec.break_end_time.is_some() {
        return Err(AppError::BadRequest("Break already ended".into()));
    }
    rec.break_end_time = Some(now);
    let duration = now.signed_duration_since(rec.break_start_time);
    rec.duration_minutes = Some(duration.num_minutes() as i32);
    rec.updated_at = now_utc;

    sqlx::query("UPDATE break_records SET break_end_time = $1, duration_minutes = $2, updated_at = $3 WHERE id = $4")
        .bind(rec.break_end_time)
        .bind(rec.duration_minutes)
        .bind(rec.updated_at)
        .bind(rec.id.to_string())
        .execute(&pool)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    if let Some(attendance) = sqlx::query_as::<_, Attendance>(
        "SELECT id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours, created_at, updated_at FROM attendance WHERE id = $1"
    )
    .bind(rec.attendance_id.to_string())
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::InternalServerError(e.into()))? {
        if attendance.clock_out_time.is_some() {
            recalculate_total_hours(&pool, attendance, now_utc).await?;
        }
    }

    Ok(Json(
        crate::models::break_record::BreakRecordResponse::from(rec),
    ))
}

pub async fn get_break_records(
    pool: &PgPool,
    attendance_id: AttendanceId,
) -> Result<Vec<BreakRecordResponse>, AppError> {
    let break_records = sqlx::query_as::<_, crate::models::break_record::BreakRecord>(
        "SELECT id, attendance_id, break_start_time, break_end_time, duration_minutes, created_at, updated_at FROM break_records WHERE attendance_id = $1 ORDER BY break_start_time"
    )
    .bind(attendance_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(break_records
        .into_iter()
        .map(BreakRecordResponse::from)
        .collect())
}
