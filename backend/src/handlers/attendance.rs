use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use chrono::{NaiveDate, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use std::str::FromStr;
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

use crate::attendance::application::commands::{
    break_end as break_end_use_case, break_start as break_start_use_case,
    build_attendance_response, clock_in as clock_in_use_case, clock_out as clock_out_use_case,
    reject_if_holiday,
};
use crate::attendance::application::queries::{
    build_attendance_status, resolve_attendance_range, resolve_summary_month,
};
use crate::attendance::application::reports::{
    build_monthly_summary, export_user_attendance, list_effective_attendance_in_range,
    AttendanceRange,
};
use crate::error::AppError;
use crate::handlers::attendance_utils::{
    ensure_authorized_access, fetch_attendance_by_id, get_break_records,
};
use crate::repositories::{
    attendance::{AttendanceRepository, AttendanceRepositoryTrait},
    break_record::BreakRecordRepository,
};
use crate::state::AppState;
use crate::types::AttendanceId;
use crate::{
    models::{
        attendance::{
            Attendance, AttendanceResponse, AttendanceSummary, ClockInRequest, ClockOutRequest,
        },
        break_record::BreakRecordResponse,
        user::User,
    },
    services::holiday::HolidayServiceTrait,
    utils::time,
};

pub type AttendanceQuery = crate::attendance::application::queries::AttendanceQuery;

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct AttendanceExportQuery {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

pub type AttendanceStatusResponse =
    crate::attendance::application::queries::AttendanceStatusResponse;

pub(crate) use crate::attendance::application::commands::recalculate_total_hours;

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

    let response = clock_in_use_case(
        &state.write_pool,
        holiday_service.as_ref(),
        user_id,
        date,
        clock_in_time,
        now_utc,
    )
    .await?;
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

    let response = clock_out_use_case(
        &state.write_pool,
        holiday_service.as_ref(),
        user_id,
        date,
        clock_out_time,
        now_utc,
    )
    .await?;
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

    let response = break_start_use_case(
        &state.write_pool,
        user.id,
        payload.attendance_id,
        break_start_time,
        now_utc,
    )
    .await?;
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

    let response = break_end_use_case(
        &state.write_pool,
        user.id,
        payload.break_record_id,
        break_end_time,
        now_utc,
    )
    .await?;
    Ok(Json(response))
}

pub async fn get_my_attendance(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(params): Query<AttendanceQuery>,
) -> Result<Json<Vec<AttendanceResponse>>, AppError> {
    let user_id = user.id;

    let tz = &state.config.time_zone;
    let (from, to) =
        resolve_attendance_range(&params, time::today_local(tz), time::now_in_timezone(tz))?;

    let responses = list_effective_attendance_in_range(
        state.read_pool(),
        user_id,
        AttendanceRange { from, to },
    )
    .await?;
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

        Ok(Json(build_attendance_status(
            Some(&att),
            active_break.as_ref(),
        )))
    } else {
        Ok(Json(build_attendance_status(None, None)))
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

    let (year, month, first_day, last_day) =
        resolve_summary_month(&params, time::now_in_timezone(&state.config.time_zone))?;

    let summary = build_monthly_summary(
        state.read_pool(),
        user_id,
        year,
        month,
        AttendanceRange {
            from: first_day,
            to: last_day,
        },
    )
    .await?;
    Ok(Json(summary))
}

pub async fn export_my_attendance(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(params): Query<AttendanceExportQuery>,
) -> Result<Json<Value>, AppError> {
    let export = export_user_attendance(
        state.read_pool(),
        user.id,
        &user.username,
        &user.full_name,
        params.from,
        params.to,
        &time::now_in_timezone(&state.config.time_zone)
            .format("%Y%m%d_%H%M%S")
            .to_string(),
    )
    .await?;
    Ok(Json(json!(export)))
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
