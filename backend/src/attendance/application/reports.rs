use chrono::{NaiveDate, NaiveDateTime};
use serde::Serialize;
use sqlx::PgPool;
use std::collections::HashMap;

use crate::{
    attendance::application::helpers::get_break_records_map,
    error::AppError,
    models::{
        attendance::{Attendance, AttendanceResponse, AttendanceSummary},
        attendance_correction_request::{AttendanceCorrectionEffectiveValue, CorrectionBreakItem},
        break_record::BreakRecordResponse,
    },
    repositories::{
        attendance::{AttendanceRepository, AttendanceRepositoryTrait},
        attendance_correction_request::AttendanceCorrectionRequestRepository,
    },
    types::{AttendanceId, BreakRecordId, UserId},
    utils::csv::append_csv_row,
};

#[derive(Debug, Clone, Copy)]
pub struct AttendanceRange {
    pub from: NaiveDate,
    pub to: NaiveDate,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct AttendanceExportResult {
    pub csv_data: String,
    pub filename: String,
}

pub async fn list_effective_attendance_in_range(
    read_pool: &PgPool,
    user_id: UserId,
    range: AttendanceRange,
) -> Result<Vec<AttendanceResponse>, AppError> {
    let repo = AttendanceRepository::new();
    let attendances = repo
        .find_by_user_and_range(read_pool, user_id, range.from, range.to)
        .await?;
    let attendance_ids: Vec<AttendanceId> =
        attendances.iter().map(|attendance| attendance.id).collect();
    let mut break_map = get_break_records_map(read_pool, &attendance_ids).await?;
    let correction_repo = AttendanceCorrectionRequestRepository::new();
    let correction_map =
        load_effective_correction_map(read_pool, &correction_repo, &attendance_ids).await?;

    let mut responses = Vec::new();
    for attendance in attendances {
        let attendance_id = attendance.id;
        let break_records = break_map.remove(&attendance_id).unwrap_or_default();
        responses.push(apply_effective_correction_to_response(
            attendance,
            break_records,
            correction_map.get(&attendance_id),
        ));
    }
    Ok(responses)
}

pub async fn build_monthly_summary(
    read_pool: &PgPool,
    user_id: UserId,
    year: i32,
    month: u32,
    range: AttendanceRange,
) -> Result<AttendanceSummary, AppError> {
    let responses = list_effective_attendance_in_range(read_pool, user_id, range).await?;
    let mut total_work_hours = 0.0;
    let mut total_work_days_i64 = 0i64;

    for response in responses {
        if let Some(hours) = response.total_work_hours {
            if hours > 0.0 {
                total_work_hours += hours;
                total_work_days_i64 += 1;
            }
        }
    }

    let total_work_days = total_work_days_i64 as i32;
    let average_daily_hours = if total_work_days > 0 {
        total_work_hours / total_work_days as f64
    } else {
        0.0
    };

    Ok(AttendanceSummary {
        month,
        year,
        total_work_hours,
        total_work_days,
        average_daily_hours,
    })
}

pub async fn export_user_attendance(
    read_pool: &PgPool,
    user_id: UserId,
    username: &str,
    full_name: &str,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    filename_suffix: &str,
) -> Result<AttendanceExportResult, AppError> {
    if let (Some(from), Some(to)) = (from, to) {
        if from > to {
            return Err(AppError::BadRequest("from must be <= to".into()));
        }
    }

    let repo = AttendanceRepository::new();
    let rows = repo
        .find_by_user_with_range_options(read_pool, user_id, from, to)
        .await?;
    let attendance_ids: Vec<AttendanceId> = rows.iter().map(|attendance| attendance.id).collect();
    let mut break_map = get_break_records_map(read_pool, &attendance_ids).await?;
    let correction_repo = AttendanceCorrectionRequestRepository::new();
    let correction_map =
        load_effective_correction_map(read_pool, &correction_repo, &attendance_ids).await?;

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
        let attendance_id = row.id;
        let breaks = break_map.remove(&attendance_id).unwrap_or_default();
        let effective =
            apply_effective_correction_to_response(row, breaks, correction_map.get(&attendance_id));

        append_csv_row(
            &mut csv_data,
            &[
                username.to_string(),
                full_name.to_string(),
                effective.date.format("%Y-%m-%d").to_string(),
                effective
                    .clock_in_time
                    .map(|time| time.format("%H:%M:%S").to_string())
                    .unwrap_or_default(),
                effective
                    .clock_out_time
                    .map(|time| time.format("%H:%M:%S").to_string())
                    .unwrap_or_default(),
                effective
                    .total_work_hours
                    .map(|hours| format!("{hours:.2}"))
                    .unwrap_or_else(|| "0.00".to_string()),
                effective.status.db_value().to_string(),
            ],
        );
    }

    Ok(AttendanceExportResult {
        csv_data,
        filename: format!("my_attendance_export_{filename_suffix}.csv"),
    })
}

pub fn apply_effective_correction_to_response(
    attendance: Attendance,
    break_records: Vec<BreakRecordResponse>,
    effective: Option<&AttendanceCorrectionEffectiveValue>,
) -> AttendanceResponse {
    if let Some(effective) = effective {
        let corrected_breaks = load_break_items_from_json(&effective.break_records_corrected_json);
        let break_records = corrected_breaks
            .iter()
            .map(|break_item| BreakRecordResponse {
                id: BreakRecordId::new(),
                attendance_id: attendance.id,
                break_start_time: break_item.break_start_time,
                break_end_time: break_item.break_end_time,
                duration_minutes: break_item.break_end_time.map(|end| {
                    end.signed_duration_since(break_item.break_start_time)
                        .num_minutes()
                        .max(0) as i32
                }),
            })
            .collect::<Vec<_>>();
        let clock_in_time = effective
            .clock_in_time_corrected
            .or(attendance.clock_in_time);
        let clock_out_time = effective
            .clock_out_time_corrected
            .or(attendance.clock_out_time);

        return AttendanceResponse {
            id: attendance.id,
            user_id: attendance.user_id,
            date: attendance.date,
            clock_in_time,
            clock_out_time,
            status: attendance.status,
            total_work_hours: calc_total_work_hours_with_breaks(
                clock_in_time,
                clock_out_time,
                &corrected_breaks,
            ),
            break_records,
        };
    }

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

pub async fn load_effective_correction_map(
    pool: &PgPool,
    repo: &AttendanceCorrectionRequestRepository,
    attendance_ids: &[AttendanceId],
) -> Result<HashMap<AttendanceId, AttendanceCorrectionEffectiveValue>, AppError> {
    let corrections = repo.get_effective_values(pool, attendance_ids).await?;
    Ok(corrections
        .into_iter()
        .map(|item| (item.attendance_id, item))
        .collect())
}

fn load_break_items_from_json(value: &serde_json::Value) -> Vec<CorrectionBreakItem> {
    serde_json::from_value(value.clone()).unwrap_or_default()
}

fn calc_total_work_hours_with_breaks(
    clock_in_time: Option<NaiveDateTime>,
    clock_out_time: Option<NaiveDateTime>,
    breaks: &[CorrectionBreakItem],
) -> Option<f64> {
    let (Some(clock_in), Some(clock_out)) = (clock_in_time, clock_out_time) else {
        return None;
    };
    let mut break_minutes = 0i64;
    for break_item in breaks {
        if let Some(end) = break_item.break_end_time {
            break_minutes += end
                .signed_duration_since(break_item.break_start_time)
                .num_minutes()
                .max(0);
        }
    }
    let gross_minutes = clock_out
        .signed_duration_since(clock_in)
        .num_minutes()
        .max(0);
    Some((gross_minutes - break_minutes).max(0) as f64 / 60.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{
            attendance::{Attendance, AttendanceStatus},
            attendance_correction_request::AttendanceCorrectionEffectiveValue,
        },
        types::{AttendanceId, UserId},
    };
    use chrono::{NaiveDate, Utc};

    #[test]
    fn attendance_export_result_keeps_filename() {
        let result = AttendanceExportResult {
            csv_data: "x".to_string(),
            filename: "file.csv".to_string(),
        };
        assert_eq!(result.filename, "file.csv");
    }

    #[test]
    fn apply_effective_correction_overrides_times() {
        let now = Utc::now();
        let date = NaiveDate::from_ymd_opt(2026, 3, 10).unwrap();
        let attendance = Attendance {
            id: AttendanceId::new(),
            user_id: UserId::new(),
            date,
            clock_in_time: Some(date.and_hms_opt(9, 0, 0).unwrap()),
            clock_out_time: Some(date.and_hms_opt(18, 0, 0).unwrap()),
            status: AttendanceStatus::Present,
            total_work_hours: Some(9.0),
            created_at: now,
            updated_at: now,
        };
        let effective = AttendanceCorrectionEffectiveValue {
            attendance_id: attendance.id,
            source_request_id: "req".to_string(),
            clock_in_time_corrected: Some(date.and_hms_opt(10, 0, 0).unwrap()),
            clock_out_time_corrected: Some(date.and_hms_opt(19, 0, 0).unwrap()),
            break_records_corrected_json: serde_json::json!([]),
            applied_by: None,
            applied_at: now,
            updated_at: now,
        };

        let response =
            apply_effective_correction_to_response(attendance, Vec::new(), Some(&effective));
        assert_eq!(response.clock_in_time, effective.clock_in_time_corrected);
        assert_eq!(response.clock_out_time, effective.clock_out_time_corrected);
    }
}
