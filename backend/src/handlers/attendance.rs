use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use chrono::{DateTime, Datelike, Duration, Months, NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::str::FromStr;
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

use crate::error::AppError;
use crate::handlers::attendance_utils::{
    ensure_authorized_access, ensure_clock_in_exists, ensure_clocked_in, ensure_not_clocked_in,
    ensure_not_clocked_out, fetch_attendance_by_id, fetch_attendance_by_user_date,
    get_break_records, get_break_records_map, insert_attendance_record, update_clock_in,
    update_clock_out,
};
use crate::repositories::{
    attendance::{AttendanceRepository, AttendanceRepositoryTrait},
    break_record::BreakRecordRepository,
    repository::Repository,
};
use crate::state::AppState;
use crate::types::{AttendanceId, UserId};
use crate::{
    models::{
        attendance::{
            Attendance, AttendanceResponse, AttendanceSummary, ClockInRequest, ClockOutRequest,
        },
        break_record::{BreakRecord, BreakRecordResponse},
        user::User,
    },
    services::holiday::HolidayServiceTrait,
    utils::{csv::append_csv_row, time},
};

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct AttendanceQuery {
    pub year: Option<i32>,
    pub month: Option<u32>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct AttendanceExportQuery {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AttendanceStatusResponse {
    pub status: String,
    pub attendance_id: Option<String>,
    pub active_break_id: Option<String>,
    pub clock_in_time: Option<NaiveDateTime>,
    pub clock_out_time: Option<NaiveDateTime>,
}

pub async fn clock_in(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Extension(holiday_service): Extension<Arc<dyn HolidayServiceTrait>>,
    Json(payload): Json<ClockInRequest>,
) -> Result<Json<AttendanceResponse>, AppError> {
    let user_id = user.id;

    let tz = &state.config.time_zone;
    let now_local = time::now_in_timezone(tz);
    let now_utc = now_local.with_timezone(&Utc);
    let date = payload.date.unwrap_or_else(|| now_local.date_naive());
    let clock_in_time = now_local.naive_local();

    reject_if_holiday(holiday_service.as_ref(), date, user_id).await?;

    let attendance: Attendance =
        match fetch_attendance_by_user_date(&state.write_pool, user_id, date).await? {
            Some(mut attendance) => {
                ensure_not_clocked_in(&attendance)?;
                attendance.clock_in_time = Some(clock_in_time);
                attendance.updated_at = now_utc;
                update_clock_in(&state.write_pool, &attendance).await?;
                attendance
            }
            None => {
                let mut attendance = Attendance::new(user_id, date, now_utc);
                attendance.clock_in_time = Some(clock_in_time);
                insert_attendance_record(&state.write_pool, &attendance).await?;
                attendance
            }
        };

    let break_records = get_break_records(&state.write_pool, attendance.id).await?;
    let response = build_attendance_response(attendance, break_records);

    Ok(Json(response))
}

pub async fn clock_out(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Extension(holiday_service): Extension<Arc<dyn HolidayServiceTrait>>,
    Json(payload): Json<ClockOutRequest>,
) -> Result<Json<AttendanceResponse>, AppError> {
    let user_id = user.id;

    let tz = &state.config.time_zone;
    let now_local = time::now_in_timezone(tz);
    let now_utc = now_local.with_timezone(&Utc);
    let date = payload.date.unwrap_or_else(|| now_local.date_naive());
    let clock_out_time = now_local.naive_local();

    reject_if_holiday(holiday_service.as_ref(), date, user_id).await?;

    let attendance_opt: Option<Attendance> =
        fetch_attendance_by_user_date(&state.write_pool, user_id, date).await?;
    let mut attendance: Attendance = attendance_opt
        .ok_or_else(|| AppError::NotFound("No attendance record found for today".into()))?;

    ensure_not_clocked_out(&attendance)?;
    ensure_clock_in_exists(&attendance)?;

    let break_repo = BreakRecordRepository::new();
    let active_break = break_repo
        .find_active_break(&state.write_pool, attendance.id)
        .await?;

    if active_break.is_some() {
        return Err(AppError::BadRequest(
            "Break in progress. End break before clocking out".into(),
        ));
    }

    attendance.clock_out_time = Some(clock_out_time);
    let break_minutes = total_break_minutes(&state.write_pool, attendance.id).await?;
    attendance.calculate_work_hours(break_minutes);
    attendance.updated_at = now_utc;

    update_clock_out(&state.write_pool, &attendance).await?;

    let break_records = get_break_records(&state.write_pool, attendance.id).await?;
    let response = build_attendance_response(attendance, break_records);

    Ok(Json(response))
}

pub async fn break_start(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<crate::models::attendance::BreakStartRequest>,
) -> Result<Json<BreakRecordResponse>, AppError> {
    let tz = &state.config.time_zone;
    let now_local = time::now_in_timezone(tz);
    let now_utc = now_local.with_timezone(&Utc);
    let break_start_time = now_local.naive_local();

    // Check if attendance record exists and user is clocked in
    let attendance = fetch_attendance_by_id(&state.write_pool, payload.attendance_id).await?;
    ensure_authorized_access(&attendance, user.id)?;
    ensure_clocked_in(&attendance)?;

    // Check if there's already an active break
    let break_repo = BreakRecordRepository::new();
    let active_break = break_repo
        .find_active_break(&state.write_pool, payload.attendance_id)
        .await?;

    if active_break.is_some() {
        return Err(AppError::BadRequest("Break already in progress".into()));
    }

    let break_record = BreakRecord::new(payload.attendance_id, break_start_time, now_utc);
    break_repo.create(&state.write_pool, &break_record).await?;

    let response = BreakRecordResponse::from(break_record);
    Ok(Json(response))
}

pub async fn break_end(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<crate::models::attendance::BreakEndRequest>,
) -> Result<Json<BreakRecordResponse>, AppError> {
    let tz = &state.config.time_zone;
    let now_local = time::now_in_timezone(tz);
    let now_utc = now_local.with_timezone(&Utc);
    let break_end_time = now_local.naive_local();

    // Find the break record
    let break_repo = BreakRecordRepository::new();
    let mut break_record = break_repo
        .find_by_id(&state.write_pool, payload.break_record_id)
        .await?;

    if !break_record.is_active() {
        return Err(AppError::BadRequest("Break already ended".into()));
    }
    let att = fetch_attendance_by_id(&state.write_pool, break_record.attendance_id).await?;
    ensure_authorized_access(&att, user.id)?;

    break_record.end_break(break_end_time, now_utc);
    break_repo.update(&state.write_pool, &break_record).await?;

    if att.clock_out_time.is_some() {
        recalculate_total_hours(&state.write_pool, att, now_utc).await?;
    }

    let response = BreakRecordResponse::from(break_record);
    Ok(Json(response))
}

pub async fn get_my_attendance(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(params): Query<AttendanceQuery>,
) -> Result<Json<Vec<AttendanceResponse>>, AppError> {
    let user_id = user.id;

    let tz = &state.config.time_zone;
    let (from, to) = if let (Some(f), Some(t)) = (params.from, params.to) {
        if f > t {
            return Err(AppError::BadRequest("from must be <= to".into()));
        }
        (f, t)
    } else if params.from.is_some() || params.to.is_some() {
        let f = params.from.unwrap_or_else(|| time::today_local(tz));
        let t = params.to.unwrap_or_else(|| time::today_local(tz));
        if f > t {
            return Err(AppError::BadRequest("from must be <= to".into()));
        }
        (f, t)
    } else {
        let now_local = time::now_in_timezone(tz);
        let year = params.year.unwrap_or_else(|| now_local.year());
        let month = params.month.unwrap_or_else(|| now_local.month());
        let Some(first_day) = NaiveDate::from_ymd_opt(year, month, 1) else {
            return Err(AppError::BadRequest("Invalid year/month provided".into()));
        };
        let Some(last_day) = first_day
            .checked_add_months(Months::new(1))
            .and_then(|d| d.checked_sub_signed(Duration::days(1)))
        else {
            return Err(AppError::BadRequest("Invalid year/month provided".into()));
        };
        (first_day, last_day)
    };

    let repo = AttendanceRepository::new();
    let attendances = repo
        .find_by_user_and_range(state.read_pool(), user_id, from, to)
        .await?;

    let attendance_ids: Vec<AttendanceId> = attendances.iter().map(|a| a.id).collect();
    let mut break_map = get_break_records_map(state.read_pool(), &attendance_ids).await?;

    let mut responses = Vec::new();
    for attendance in attendances {
        let break_records = break_map.remove(&attendance.id).unwrap_or_default();
        responses.push(build_attendance_response(attendance, break_records));
    }

    Ok(Json(responses))
}

pub async fn get_attendance_status(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<AttendanceStatusResponse>, AppError> {
    let user_id = user.id;
    let date = params
        .get("date")
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| time::today_local(&state.config.time_zone));

    let repo = AttendanceRepository::new();
    let attendance = repo
        .find_by_user_and_date(state.read_pool(), user_id, date)
        .await?;

    if let Some(att) = attendance {
        // Check active break
        let break_repo = BreakRecordRepository::new();
        let active_break = break_repo
            .find_active_break(state.read_pool(), att.id)
            .await?;

        let resp = if att.clock_in_time.is_none() {
            AttendanceStatusResponse {
                status: "not_started".into(),
                attendance_id: Some(att.id.to_string()),
                active_break_id: None,
                clock_in_time: None,
                clock_out_time: None,
            }
        } else if att.is_clocked_out() {
            AttendanceStatusResponse {
                status: "clocked_out".into(),
                attendance_id: Some(att.id.to_string()),
                active_break_id: None,
                clock_in_time: att.clock_in_time,
                clock_out_time: att.clock_out_time,
            }
        } else if let Some(b) = active_break {
            let bid: Option<String> = Some(b.id.to_string());
            AttendanceStatusResponse {
                status: "on_break".into(),
                attendance_id: Some(att.id.to_string()),
                active_break_id: bid,
                clock_in_time: att.clock_in_time,
                clock_out_time: None,
            }
        } else {
            AttendanceStatusResponse {
                status: "clocked_in".into(),
                attendance_id: Some(att.id.to_string()),
                active_break_id: None,
                clock_in_time: att.clock_in_time,
                clock_out_time: None,
            }
        };
        Ok(Json(resp))
    } else {
        Ok(Json(AttendanceStatusResponse {
            status: "not_started".into(),
            attendance_id: None,
            active_break_id: None,
            clock_in_time: None,
            clock_out_time: None,
        }))
    }
}

pub async fn get_breaks_by_attendance(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(attendance_id): Path<String>,
) -> Result<Json<Vec<BreakRecordResponse>>, AppError> {
    let attendance_id = AttendanceId::from_str(&attendance_id)
        .map_err(|_| AppError::BadRequest("Invalid attendance ID format".into()))?;
    let attendance = fetch_attendance_by_id(state.read_pool(), attendance_id).await?;
    ensure_authorized_access(&attendance, user.id)?;
    let records = get_break_records(state.read_pool(), attendance.id).await?;
    Ok(Json(records))
}

pub async fn get_my_summary(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(params): Query<AttendanceQuery>,
) -> Result<Json<AttendanceSummary>, AppError> {
    let user_id = user.id;

    let now_local = time::now_in_timezone(&state.config.time_zone);
    let year = params.year.unwrap_or_else(|| now_local.year());
    let month = params.month.unwrap_or_else(|| now_local.month());

    let Some(first_day) = NaiveDate::from_ymd_opt(year, month, 1) else {
        return Err(AppError::BadRequest("Invalid year/month provided".into()));
    };
    let Some(last_day) = first_day
        .checked_add_months(Months::new(1))
        .and_then(|d| d.checked_sub_signed(Duration::days(1)))
    else {
        return Err(AppError::BadRequest("Invalid year/month provided".into()));
    };

    let repo = AttendanceRepository::new();
    let (total_work_hours, total_work_days_i64) = repo
        .get_summary_stats(state.read_pool(), user_id, first_day, last_day)
        .await?;

    let total_work_days = total_work_days_i64 as i32;
    let average_daily_hours = if total_work_days > 0 {
        total_work_hours / total_work_days as f64
    } else {
        0.0
    };

    let summary = AttendanceSummary {
        month,
        year,
        total_work_hours,
        total_work_days,
        average_daily_hours,
    };

    Ok(Json(summary))
}

pub async fn export_my_attendance(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(params): Query<AttendanceExportQuery>,
) -> Result<Json<Value>, AppError> {
    let from = params.from;
    let to = params.to;

    if let (Some(f), Some(t)) = (from, to) {
        if f > t {
            return Err(AppError::BadRequest("from must be <= to".into()));
        }
    }

    let repo = AttendanceRepository::new();
    let rows = repo
        .find_by_user_with_range_options(state.read_pool(), user.id, from, to)
        .await?;

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
    for row in rows {
        let username = user.username.clone();
        let full_name = user.full_name.clone();
        let date = row.date.format("%Y-%m-%d").to_string();
        let clock_in = row
            .clock_in_time
            .map(|t| t.format("%H:%M:%S").to_string())
            .unwrap_or_default();
        let clock_out = row
            .clock_out_time
            .map(|t| t.format("%H:%M:%S").to_string())
            .unwrap_or_default();
        let total_hours = row
            .total_work_hours
            .map(|h| format!("{:.2}", h))
            .unwrap_or_else(|| "0.00".to_string());
        let status = row.status.db_value().to_string();

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
            "my_attendance_export_{}.csv",
            time::now_in_timezone(&state.config.time_zone).format("%Y%m%d_%H%M%S")
        )
    })))
}

fn build_attendance_response(
    attendance: Attendance,
    break_records: Vec<BreakRecordResponse>,
) -> AttendanceResponse {
    AttendanceResponse {
        id: attendance.id,
        user_id: attendance.user_id,
        date: attendance.date,
        clock_in_time: attendance.clock_in_time,
        clock_out_time: attendance.clock_out_time,
        status: attendance.status,
        total_work_hours: attendance.total_work_hours,
        break_records,
    }
}

pub(crate) async fn total_break_minutes(
    pool: &PgPool,
    attendance_id: AttendanceId,
) -> Result<i64, AppError> {
    let repo = BreakRecordRepository::new();
    repo.get_total_duration(pool, attendance_id).await
}

pub(crate) async fn recalculate_total_hours(
    pool: &PgPool,
    mut attendance: Attendance,
    updated_at: DateTime<Utc>,
) -> Result<(), AppError> {
    if attendance.clock_in_time.is_none() || attendance.clock_out_time.is_none() {
        return Ok(());
    }

    let break_repo = BreakRecordRepository::new();
    let break_minutes = break_repo.get_total_duration(pool, attendance.id).await?;

    attendance.calculate_work_hours(break_minutes);
    attendance.updated_at = updated_at;

    let att_repo = AttendanceRepository::new();
    att_repo.update(pool, &attendance).await?;

    Ok(())
}

async fn reject_if_holiday(
    holiday_service: &dyn HolidayServiceTrait,
    date: NaiveDate,
    user_id: UserId,
) -> Result<(), AppError> {
    let decision = holiday_service
        .is_holiday(date, Some(&user_id.to_string()))
        .await?;

    if decision.is_holiday {
        let reason = decision.reason.label();
        return Err(AppError::Forbidden(format!(
            "{} is a {}. Submit an overtime request before clocking in/out.",
            date, reason
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::holiday::{HolidayCalendarEntry, HolidayDecision, HolidayReason};
    use crate::types::{AttendanceId, UserId};
    use chrono::NaiveDate;
    use sqlx::postgres::PgPoolOptions;
    use std::sync::Arc;

    #[test]
    fn test_attendance_query_default_values() {
        let query = AttendanceQuery {
            year: None,
            month: None,
            from: None,
            to: None,
        };
        assert!(query.year.is_none());
        assert!(query.month.is_none());
        assert!(query.from.is_none());
        assert!(query.to.is_none());
    }

    #[test]
    fn test_attendance_query_with_values() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let query = AttendanceQuery {
            year: Some(2024),
            month: Some(1),
            from: Some(date),
            to: Some(date),
        };
        assert_eq!(query.year, Some(2024));
        assert_eq!(query.month, Some(1));
        assert_eq!(query.from, Some(date));
        assert_eq!(query.to, Some(date));
    }

    #[test]
    fn test_attendance_export_query_default_values() {
        let query = AttendanceExportQuery {
            from: None,
            to: None,
        };
        assert!(query.from.is_none());
        assert!(query.to.is_none());
    }

    #[test]
    fn test_attendance_status_response_structure() {
        let response = AttendanceStatusResponse {
            status: "clocked_in".to_string(),
            attendance_id: Some("test-id".to_string()),
            active_break_id: None,
            clock_in_time: None,
            clock_out_time: None,
        };
        assert_eq!(response.status, "clocked_in");
        assert!(response.attendance_id.is_some());
        assert!(response.active_break_id.is_none());
    }

    #[test]
    fn test_clock_in_request_structure() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 15);
        let request = ClockInRequest { date };
        assert_eq!(request.date, date);
    }

    #[test]
    fn test_clock_out_request_structure() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 15);
        let request = ClockOutRequest { date };
        assert_eq!(request.date, date);
    }

    #[test]
    fn test_attendance_summary_structure() {
        let summary = AttendanceSummary {
            month: 1,
            year: 2024,
            total_work_hours: 160.5,
            total_work_days: 20,
            average_daily_hours: 8.0,
        };
        assert_eq!(summary.month, 1);
        assert_eq!(summary.year, 2024);
        assert_eq!(summary.total_work_hours, 160.5);
        assert_eq!(summary.total_work_days, 20);
        assert_eq!(summary.average_daily_hours, 8.0);
    }

    struct FixedHolidayService {
        decision: HolidayDecision,
    }

    #[async_trait::async_trait]
    impl crate::services::holiday::HolidayServiceTrait for FixedHolidayService {
        async fn is_holiday(
            &self,
            _date: NaiveDate,
            _user_id: Option<&str>,
        ) -> sqlx::Result<HolidayDecision> {
            Ok(self.decision.clone())
        }

        async fn list_month(
            &self,
            _year: i32,
            _month: u32,
            _user_id: Option<&str>,
        ) -> sqlx::Result<Vec<HolidayCalendarEntry>> {
            Ok(Vec::new())
        }
    }

    #[tokio::test]
    async fn reject_if_holiday_allows_working_day() {
        let service = Arc::new(FixedHolidayService {
            decision: HolidayDecision {
                is_holiday: false,
                reason: HolidayReason::None,
            },
        });
        let date = NaiveDate::from_ymd_opt(2026, 2, 4).expect("date");
        let user_id = UserId::new();

        let result = reject_if_holiday(service.as_ref(), date, user_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn reject_if_holiday_rejects_holiday_with_reason() {
        let service = Arc::new(FixedHolidayService {
            decision: HolidayDecision {
                is_holiday: true,
                reason: HolidayReason::PublicHoliday,
            },
        });
        let date = NaiveDate::from_ymd_opt(2026, 2, 11).expect("date");
        let user_id = UserId::new();

        let result = reject_if_holiday(service.as_ref(), date, user_id).await;
        let err = result.expect_err("holiday should be rejected");
        match err {
            AppError::Forbidden(message) => assert!(message.contains("public holiday")),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn build_attendance_response_keeps_core_fields() {
        let now = Utc::now();
        let attendance = Attendance {
            id: AttendanceId::new(),
            user_id: UserId::new(),
            date: NaiveDate::from_ymd_opt(2026, 2, 4).expect("date"),
            clock_in_time: Some(
                chrono::NaiveDateTime::parse_from_str("2026-02-04T09:00:00", "%Y-%m-%dT%H:%M:%S")
                    .expect("clock in"),
            ),
            clock_out_time: None,
            status: crate::models::attendance::AttendanceStatus::Present,
            total_work_hours: None,
            created_at: now,
            updated_at: now,
        };

        let response = build_attendance_response(attendance.clone(), Vec::new());
        assert_eq!(response.id, attendance.id);
        assert_eq!(response.user_id, attendance.user_id);
        assert_eq!(response.date, attendance.date);
        assert_eq!(response.clock_in_time, attendance.clock_in_time);
        assert_eq!(response.clock_out_time, attendance.clock_out_time);
        assert!(response.break_records.is_empty());
    }

    #[tokio::test]
    async fn recalculate_total_hours_returns_early_when_times_missing() {
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://127.0.0.1:15432/timekeeper")
            .expect("lazy pool");
        let now = Utc::now();

        let mut attendance = Attendance::new(
            UserId::new(),
            NaiveDate::from_ymd_opt(2026, 2, 4).expect("date"),
            now,
        );
        attendance.clock_in_time = None;
        attendance.clock_out_time = None;

        let result = recalculate_total_hours(&pool, attendance, now).await;
        assert!(result.is_ok());
    }
}
