use chrono::{Datelike, Duration, Months, NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{
    error::AppError,
    models::{attendance::Attendance, break_record::BreakRecord},
};

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct AttendanceQuery {
    pub year: Option<i32>,
    pub month: Option<u32>,
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

pub fn resolve_attendance_range(
    params: &AttendanceQuery,
    today: NaiveDate,
    now_local: chrono::DateTime<chrono_tz::Tz>,
) -> Result<(NaiveDate, NaiveDate), AppError> {
    if let (Some(from), Some(to)) = (params.from, params.to) {
        if from > to {
            return Err(AppError::BadRequest("from must be <= to".into()));
        }
        return Ok((from, to));
    }

    if params.from.is_some() || params.to.is_some() {
        let from = params.from.unwrap_or(today);
        let to = params.to.unwrap_or(today);
        if from > to {
            return Err(AppError::BadRequest("from must be <= to".into()));
        }
        return Ok((from, to));
    }

    let year = params.year.unwrap_or_else(|| now_local.year());
    let month = params.month.unwrap_or_else(|| now_local.month());
    resolve_month_bounds(year, month)
}

pub fn resolve_summary_month(
    params: &AttendanceQuery,
    now_local: chrono::DateTime<chrono_tz::Tz>,
) -> Result<(i32, u32, NaiveDate, NaiveDate), AppError> {
    let year = params.year.unwrap_or_else(|| now_local.year());
    let month = params.month.unwrap_or_else(|| now_local.month());
    let (first_day, last_day) = resolve_month_bounds(year, month)?;
    Ok((year, month, first_day, last_day))
}

pub fn build_attendance_status(
    attendance: Option<&Attendance>,
    active_break: Option<&BreakRecord>,
) -> AttendanceStatusResponse {
    match attendance {
        Some(attendance) if attendance.clock_in_time.is_none() => AttendanceStatusResponse {
            status: "not_started".into(),
            attendance_id: Some(attendance.id.to_string()),
            active_break_id: None,
            clock_in_time: None,
            clock_out_time: None,
        },
        Some(attendance) if attendance.is_clocked_out() => AttendanceStatusResponse {
            status: "clocked_out".into(),
            attendance_id: Some(attendance.id.to_string()),
            active_break_id: None,
            clock_in_time: attendance.clock_in_time,
            clock_out_time: attendance.clock_out_time,
        },
        Some(attendance) => {
            if let Some(active_break) = active_break {
                return AttendanceStatusResponse {
                    status: "on_break".into(),
                    attendance_id: Some(attendance.id.to_string()),
                    active_break_id: Some(active_break.id.to_string()),
                    clock_in_time: attendance.clock_in_time,
                    clock_out_time: None,
                };
            }
            AttendanceStatusResponse {
                status: "clocked_in".into(),
                attendance_id: Some(attendance.id.to_string()),
                active_break_id: None,
                clock_in_time: attendance.clock_in_time,
                clock_out_time: None,
            }
        }
        None => AttendanceStatusResponse {
            status: "not_started".into(),
            attendance_id: None,
            active_break_id: None,
            clock_in_time: None,
            clock_out_time: None,
        },
    }
}

fn resolve_month_bounds(year: i32, month: u32) -> Result<(NaiveDate, NaiveDate), AppError> {
    let Some(first_day) = NaiveDate::from_ymd_opt(year, month, 1) else {
        return Err(AppError::BadRequest("Invalid year/month provided".into()));
    };
    let Some(last_day) = first_day
        .checked_add_months(Months::new(1))
        .and_then(|day| day.checked_sub_signed(Duration::days(1)))
    else {
        return Err(AppError::BadRequest("Invalid year/month provided".into()));
    };
    Ok((first_day, last_day))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{
            attendance::{Attendance, AttendanceStatus},
            break_record::BreakRecord,
        },
        types::{AttendanceId, UserId},
    };
    use chrono::{TimeZone, Utc};
    use chrono_tz::Asia::Tokyo;

    #[test]
    fn resolve_attendance_range_uses_explicit_range() {
        let from = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2026, 3, 31).unwrap();
        let params = AttendanceQuery {
            year: None,
            month: None,
            from: Some(from),
            to: Some(to),
        };

        let result = resolve_attendance_range(
            &params,
            from,
            Tokyo.with_ymd_and_hms(2026, 3, 15, 9, 0, 0).unwrap(),
        )
        .unwrap();
        assert_eq!(result, (from, to));
    }

    #[test]
    fn resolve_attendance_range_rejects_inverted_range() {
        let params = AttendanceQuery {
            year: None,
            month: None,
            from: Some(NaiveDate::from_ymd_opt(2026, 3, 31).unwrap()),
            to: Some(NaiveDate::from_ymd_opt(2026, 3, 1).unwrap()),
        };
        assert!(matches!(
            resolve_attendance_range(
                &params,
                NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
                Tokyo.with_ymd_and_hms(2026, 3, 15, 9, 0, 0).unwrap()
            ),
            Err(AppError::BadRequest(_))
        ));
    }

    #[test]
    fn resolve_summary_month_uses_current_month_by_default() {
        let params = AttendanceQuery {
            year: None,
            month: None,
            from: None,
            to: None,
        };
        let result = resolve_summary_month(
            &params,
            Tokyo.with_ymd_and_hms(2026, 3, 15, 9, 0, 0).unwrap(),
        )
        .unwrap();
        assert_eq!(result.0, 2026);
        assert_eq!(result.1, 3);
        assert_eq!(result.2, NaiveDate::from_ymd_opt(2026, 3, 1).unwrap());
        assert_eq!(result.3, NaiveDate::from_ymd_opt(2026, 3, 31).unwrap());
    }

    #[test]
    fn build_attendance_status_marks_break_state() {
        let now = Utc.with_ymd_and_hms(2026, 3, 15, 0, 0, 0).unwrap();
        let date = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
        let attendance = Attendance {
            id: AttendanceId::new(),
            user_id: UserId::new(),
            date,
            clock_in_time: Some(date.and_hms_opt(9, 0, 0).unwrap()),
            clock_out_time: None,
            status: AttendanceStatus::Present,
            total_work_hours: None,
            created_at: now,
            updated_at: now,
        };
        let break_record =
            BreakRecord::new(attendance.id, date.and_hms_opt(12, 0, 0).unwrap(), now);
        let response = build_attendance_status(Some(&attendance), Some(&break_record));
        assert_eq!(response.status, "on_break");
        assert_eq!(response.attendance_id, Some(attendance.id.to_string()));
        assert_eq!(response.active_break_id, Some(break_record.id.to_string()));
    }
}
