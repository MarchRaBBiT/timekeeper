use std::sync::Mutex;

use chrono::{NaiveDate, NaiveDateTime};
use timekeeper_app::attendance::{
    AttendanceRecord, BreakPeriod, ListAttendancePage, ListAttendancePageError,
    ListAttendancePageQuery, ListAttendancePageRepository,
};

#[derive(Default)]
struct RecordingAttendancePageRepository {
    total: Mutex<i64>,
    attendances: Mutex<Vec<AttendanceRecord>>,
    break_periods: Mutex<Vec<BreakPeriod>>,
    requested_ids: Mutex<Vec<String>>,
}

#[async_trait::async_trait]
impl ListAttendancePageRepository for RecordingAttendancePageRepository {
    async fn count_attendance(&self) -> Result<i64, ListAttendancePageError> {
        Ok(*self.total.lock().expect("total lock"))
    }

    async fn list_attendance(
        &self,
        _limit: i64,
        _offset: i64,
    ) -> Result<Vec<AttendanceRecord>, ListAttendancePageError> {
        Ok(self.attendances.lock().expect("attendances lock").clone())
    }

    async fn breaks_for_attendance_ids(
        &self,
        attendance_ids: &[String],
    ) -> Result<Vec<BreakPeriod>, ListAttendancePageError> {
        *self.requested_ids.lock().expect("requested ids lock") = attendance_ids.to_vec();
        Ok(self
            .break_periods
            .lock()
            .expect("break periods lock")
            .clone())
    }
}

fn date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 6, 12).expect("date")
}

fn time(hour: u32) -> NaiveDateTime {
    date().and_hms_opt(hour, 0, 0).expect("time")
}

fn attendance_record(id: &str) -> AttendanceRecord {
    AttendanceRecord {
        attendance_id: id.to_string(),
        user_id: format!("user-{id}"),
        date: date(),
        clock_in_time: Some(time(9)),
        clock_out_time: Some(time(18)),
        status: "completed".to_string(),
        total_work_hours: Some(8.0),
    }
}

fn break_period(id: &str, attendance_id: &str, hour: u32) -> BreakPeriod {
    BreakPeriod {
        break_id: id.to_string(),
        attendance_id: attendance_id.to_string(),
        break_start_time: time(hour),
        break_end_time: Some(time(hour + 1)),
        duration_minutes: Some(60),
    }
}

#[tokio::test]
async fn lists_attendance_page_with_break_periods() {
    let repository = RecordingAttendancePageRepository::default();
    *repository.total.lock().expect("total lock") = 12;
    *repository.attendances.lock().expect("attendances lock") = vec![
        attendance_record("attendance-1"),
        attendance_record("attendance-2"),
    ];
    *repository.break_periods.lock().expect("break periods lock") =
        vec![break_period("break-1", "attendance-2", 12)];
    let use_case = ListAttendancePage::new(repository);

    let page = use_case
        .execute(ListAttendancePageQuery {
            limit: 10,
            offset: 20,
        })
        .await
        .expect("list succeeds");

    assert_eq!(page.total, 12);
    assert_eq!(page.limit, 10);
    assert_eq!(page.offset, 20);
    assert_eq!(page.items.len(), 2);
    assert!(page.items[0].break_periods.is_empty());
    assert_eq!(page.items[1].break_periods[0].break_id, "break-1");
}

#[tokio::test]
async fn returns_empty_items_without_requesting_breaks_for_empty_page() {
    let repository = RecordingAttendancePageRepository::default();
    let use_case = ListAttendancePage::new(repository);

    let page = use_case
        .execute(ListAttendancePageQuery {
            limit: 10,
            offset: 0,
        })
        .await
        .expect("list succeeds");

    assert!(page.items.is_empty());
}
