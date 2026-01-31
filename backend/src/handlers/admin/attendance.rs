use crate::error::AppError;
use crate::models::{PaginatedResponse, PaginationQuery};
use crate::types::{AttendanceId, BreakRecordId, UserId};
use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use std::str::FromStr;
use utoipa::ToSchema;

use crate::repositories::attendance::AttendanceRepository;
use crate::repositories::break_record::BreakRecordRepository;
use crate::repositories::repository::Repository;
use crate::repositories::transaction;
use crate::state::AppState;
use crate::{
    handlers::attendance::recalculate_total_hours,
    handlers::attendance_utils::{get_break_records, get_break_records_map},
    models::{attendance::AttendanceResponse, user::User},
    utils::time,
};

pub async fn get_all_attendance(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<PaginatedResponse<AttendanceResponse>>, AppError> {
    if !user.is_system_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    let limit = pagination.limit();
    let offset = pagination.offset();

    let repo = AttendanceRepository::new();
    let total = repo.count_all(state.read_pool()).await?;

    let attendances = repo
        .list_paginated(state.read_pool(), limit, offset)
        .await?;

    let attendance_ids: Vec<AttendanceId> = attendances.iter().map(|a| a.id).collect();
    let mut break_map = get_break_records_map(state.read_pool(), &attendance_ids).await?;

    let mut data = Vec::new();
    for attendance in attendances {
        let break_records = break_map.remove(&attendance.id).unwrap_or_default();
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
        data.push(response);
    }

    Ok(Json(PaginatedResponse::new(data, total, limit, offset)))
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
    State(state): State<AppState>,
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

    let date = NaiveDate::parse_from_str(&date, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("Invalid date".into()))?;
    let cin = NaiveDateTime::parse_from_str(&clock_in_time, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(&clock_in_time, "%Y-%m-%d %H:%M:%S"))
        .map_err(|_| AppError::BadRequest("Invalid clock_in_time".into()))?;
    let cout = match &clock_out_time {
        Some(s) => Some(
            NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
                .map_err(|_| AppError::BadRequest("Invalid clock_out_time".into()))?,
        ),
        None => None,
    };

    let mut tx = transaction::begin_transaction(&state.write_pool).await?;

    // Parse and validate user_id
    let user_id_typed = UserId::from_str(&user_id)
        .map_err(|_| AppError::BadRequest("Invalid user_id format".into()))?;

    let attendance_repo = AttendanceRepository::new();
    let break_repo = BreakRecordRepository::new();

    // ensure unique per user/date: delete existing and reinsert (basic upsert)
    attendance_repo
        .delete_by_user_and_date(&mut tx, user_id_typed, date)
        .await?;

    let mut att = crate::models::attendance::Attendance::new(
        user_id_typed,
        date,
        time::now_utc(&state.config.time_zone),
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
                    .map_err(|_| AppError::BadRequest("Invalid break_start_time".into()))?;
            let be: Option<chrono::NaiveDateTime> = b.break_end_time.as_ref().and_then(|s| {
                chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
                    .ok()
                    .or_else(|| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").ok())
            });
            let now_utc = time::now_utc(&state.config.time_zone);
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

    attendance_repo.create_in_transaction(&mut tx, &att).await?;

    // insert breaks
    for br in pending_breaks {
        break_repo.create_in_transaction(&mut tx, &br).await?;
    }

    transaction::commit_transaction(tx).await?;

    let breaks = get_break_records(&state.write_pool, att.id).await?;
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
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(break_id): Path<String>,
) -> Result<Json<crate::models::break_record::BreakRecordResponse>, AppError> {
    if !user.is_system_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }
    let break_record_id = BreakRecordId::from_str(&break_id)
        .map_err(|_| AppError::BadRequest("Invalid break record ID format".into()))?;
    let now_local = time::now_in_timezone(&state.config.time_zone);
    let now_utc = now_local.with_timezone(&Utc);
    let now = now_local.naive_local();
    let break_repo = BreakRecordRepository::new();
    let mut rec = break_repo
        .find_by_id(&state.write_pool, break_record_id)
        .await?;

    if rec.break_end_time.is_some() {
        return Err(AppError::BadRequest("Break already ended".into()));
    }
    rec.break_end_time = Some(now);
    let duration = now.signed_duration_since(rec.break_start_time);
    rec.duration_minutes = Some(duration.num_minutes() as i32);
    rec.updated_at = now_utc;

    break_repo.update(&state.write_pool, &rec).await?;

    let attendance_repo = AttendanceRepository::new();
    if let Some(attendance) = attendance_repo
        .find_optional_by_id(&state.write_pool, rec.attendance_id)
        .await?
    {
        if attendance.clock_out_time.is_some() {
            recalculate_total_hours(&state.write_pool, attendance, now_utc).await?;
        }
    }

    Ok(Json(
        crate::models::break_record::BreakRecordResponse::from(rec),
    ))
}
