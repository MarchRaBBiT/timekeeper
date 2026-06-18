use std::sync::Mutex;

use chrono::{NaiveDate, NaiveDateTime};
use timekeeper_app::attendance::{
    AttendanceRecord, BreakPeriod, EffectiveAttendanceCorrection, ExportUserAttendance,
    ExportUserAttendanceQuery, GetUserAttendanceSummary, GetUserAttendanceSummaryQuery,
    ListUserAttendance, ListUserAttendanceError, ListUserAttendanceQuery,
    ListUserAttendanceRepository,
};

#[derive(Default)]
struct RecordingUserAttendanceRepository {
    attendances: Mutex<Vec<AttendanceRecord>>,
    break_periods: Mutex<Vec<BreakPeriod>>,
    corrections: Mutex<Vec<EffectiveAttendanceCorrection>>,
}

#[async_trait::async_trait]
impl ListUserAttendanceRepository for RecordingUserAttendanceRepository {
    async fn list_user_attendance(
        &self,
        _user_id: &str,
        _from: NaiveDate,
        _to: NaiveDate,
    ) -> Result<Vec<AttendanceRecord>, ListUserAttendanceError> {
        Ok(self.attendances.lock().expect("attendances lock").clone())
    }

    async fn breaks_for_attendance_ids(
        &self,
        _attendance_ids: &[String],
    ) -> Result<Vec<BreakPeriod>, ListUserAttendanceError> {
        Ok(self
            .break_periods
            .lock()
            .expect("break periods lock")
            .clone())
    }

    async fn effective_corrections_for_attendance_ids(
        &self,
        _attendance_ids: &[String],
    ) -> Result<Vec<EffectiveAttendanceCorrection>, ListUserAttendanceError> {
        Ok(self.corrections.lock().expect("corrections lock").clone())
    }

    async fn list_user_attendance_with_optional_range(
        &self,
        _user_id: &str,
        _from: Option<NaiveDate>,
        _to: Option<NaiveDate>,
    ) -> Result<Vec<AttendanceRecord>, ListUserAttendanceError> {
        Ok(self.attendances.lock().expect("attendances lock").clone())
    }
}

fn date(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 6, day).expect("date")
}

fn time(day: u32, hour: u32) -> NaiveDateTime {
    date(day).and_hms_opt(hour, 0, 0).expect("time")
}

fn attendance_record(id: &str, day: u32) -> AttendanceRecord {
    AttendanceRecord {
        attendance_id: id.to_string(),
        user_id: "user-1".to_string(),
        date: date(day),
        clock_in_time: Some(time(day, 9)),
        clock_out_time: Some(time(day, 18)),
        status: "present".to_string(),
        total_work_hours: Some(8.0),
    }
}

fn break_period(id: &str, attendance_id: &str, day: u32, start_hour: u32) -> BreakPeriod {
    BreakPeriod {
        break_id: id.to_string(),
        attendance_id: attendance_id.to_string(),
        break_start_time: time(day, start_hour),
        break_end_time: Some(time(day, start_hour + 1)),
        duration_minutes: Some(60),
    }
}

#[tokio::test]
async fn lists_user_attendance_with_breaks() {
    let repository = RecordingUserAttendanceRepository::default();
    *repository.attendances.lock().expect("attendances lock") =
        vec![attendance_record("attendance-1", 12)];
    *repository.break_periods.lock().expect("break periods lock") =
        vec![break_period("break-1", "attendance-1", 12, 12)];
    let use_case = ListUserAttendance::new(repository);

    let items = use_case
        .execute(ListUserAttendanceQuery {
            user_id: "user-1".to_string(),
            from: date(1),
            to: date(30),
        })
        .await
        .expect("list succeeds");

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].attendance.attendance_id, "attendance-1");
    assert_eq!(items[0].break_periods[0].break_id, "break-1");
}

#[tokio::test]
async fn applies_effective_correction_to_attendance_and_breaks() {
    let repository = RecordingUserAttendanceRepository::default();
    *repository.attendances.lock().expect("attendances lock") =
        vec![attendance_record("attendance-1", 12)];
    *repository.break_periods.lock().expect("break periods lock") =
        vec![break_period("original-break", "attendance-1", 12, 12)];
    *repository.corrections.lock().expect("corrections lock") =
        vec![EffectiveAttendanceCorrection {
            attendance_id: "attendance-1".to_string(),
            clock_in_time_corrected: Some(time(12, 10)),
            clock_out_time_corrected: Some(time(12, 17)),
            corrected_breaks: vec![break_period("corrected-break", "attendance-1", 12, 13)],
        }];
    let use_case = ListUserAttendance::new(repository);

    let items = use_case
        .execute(ListUserAttendanceQuery {
            user_id: "user-1".to_string(),
            from: date(1),
            to: date(30),
        })
        .await
        .expect("list succeeds");

    assert_eq!(items[0].attendance.clock_in_time, Some(time(12, 10)));
    assert_eq!(items[0].attendance.clock_out_time, Some(time(12, 17)));
    assert_eq!(items[0].attendance.total_work_hours, Some(6.0));
    assert_eq!(items[0].break_periods[0].break_id, "corrected-break");
}

#[tokio::test]
async fn summarizes_positive_work_days() {
    let repository = RecordingUserAttendanceRepository::default();
    *repository.attendances.lock().expect("attendances lock") = vec![
        AttendanceRecord {
            total_work_hours: Some(8.0),
            ..attendance_record("attendance-1", 12)
        },
        AttendanceRecord {
            total_work_hours: Some(0.0),
            ..attendance_record("attendance-2", 13)
        },
        AttendanceRecord {
            total_work_hours: Some(6.0),
            ..attendance_record("attendance-3", 14)
        },
    ];
    let use_case = GetUserAttendanceSummary::new(repository);

    let summary = use_case
        .execute(GetUserAttendanceSummaryQuery {
            user_id: "user-1".to_string(),
            year: 2026,
            month: 6,
            from: date(1),
            to: date(30),
        })
        .await
        .expect("summary succeeds");

    assert_eq!(summary.total_work_hours, 14.0);
    assert_eq!(summary.total_work_days, 2);
    assert_eq!(summary.average_daily_hours, 7.0);
}

#[tokio::test]
async fn summarizes_effective_correction_totals() {
    let repository = RecordingUserAttendanceRepository::default();
    *repository.attendances.lock().expect("attendances lock") =
        vec![attendance_record("attendance-1", 12)];
    *repository.corrections.lock().expect("corrections lock") =
        vec![EffectiveAttendanceCorrection {
            attendance_id: "attendance-1".to_string(),
            clock_in_time_corrected: Some(time(12, 10)),
            clock_out_time_corrected: Some(time(12, 17)),
            corrected_breaks: vec![break_period("corrected-break", "attendance-1", 12, 13)],
        }];
    let use_case = GetUserAttendanceSummary::new(repository);

    let summary = use_case
        .execute(GetUserAttendanceSummaryQuery {
            user_id: "user-1".to_string(),
            year: 2026,
            month: 6,
            from: date(1),
            to: date(30),
        })
        .await
        .expect("summary succeeds");

    assert_eq!(summary.total_work_hours, 6.0);
    assert_eq!(summary.total_work_days, 1);
    assert_eq!(summary.average_daily_hours, 6.0);
}

#[tokio::test]
async fn exports_user_attendance_rows() {
    let repository = RecordingUserAttendanceRepository::default();
    *repository.attendances.lock().expect("attendances lock") =
        vec![attendance_record("attendance-1", 12)];
    let use_case = ExportUserAttendance::new(repository);

    let export = use_case
        .execute(ExportUserAttendanceQuery {
            user_id: "user-1".to_string(),
            username: "employee".to_string(),
            full_name: "Test User".to_string(),
            from: None,
            to: None,
        })
        .await
        .expect("export succeeds");

    assert_eq!(export.rows.len(), 1);
    assert_eq!(export.rows[0].username, "employee");
    assert_eq!(export.rows[0].full_name, "Test User");
    assert_eq!(export.rows[0].date, date(12));
    assert_eq!(export.rows[0].total_work_hours, Some(8.0));
}

#[tokio::test]
async fn exports_effective_correction_values() {
    let repository = RecordingUserAttendanceRepository::default();
    *repository.attendances.lock().expect("attendances lock") =
        vec![attendance_record("attendance-1", 12)];
    *repository.corrections.lock().expect("corrections lock") =
        vec![EffectiveAttendanceCorrection {
            attendance_id: "attendance-1".to_string(),
            clock_in_time_corrected: Some(time(12, 10)),
            clock_out_time_corrected: Some(time(12, 17)),
            corrected_breaks: vec![break_period("corrected-break", "attendance-1", 12, 13)],
        }];
    let use_case = ExportUserAttendance::new(repository);

    let export = use_case
        .execute(ExportUserAttendanceQuery {
            user_id: "user-1".to_string(),
            username: "employee".to_string(),
            full_name: "Test User".to_string(),
            from: Some(date(1)),
            to: Some(date(30)),
        })
        .await
        .expect("export succeeds");

    assert_eq!(export.rows[0].clock_in_time, Some(time(12, 10)));
    assert_eq!(export.rows[0].clock_out_time, Some(time(12, 17)));
    assert_eq!(export.rows[0].total_work_hours, Some(6.0));
}
