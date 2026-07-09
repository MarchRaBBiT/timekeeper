use std::sync::Mutex;

use chrono::{NaiveDate, NaiveDateTime};
use timekeeper_app::attendance::{
    AdminAttendanceExportFilters, AdminAttendanceExportRow, AdminLeaveDayRow,
    ExportAdminAttendance, ExportAdminAttendanceError, ExportAdminAttendanceQuery,
    ExportAdminAttendanceRepository, ON_LEAVE_STATUS,
};

#[derive(Default)]
struct RecordingAdminExportRepository {
    subordinate_ids: Mutex<Vec<String>>,
    rows: Mutex<Vec<AdminAttendanceExportRow>>,
    leave_days: Mutex<Vec<AdminLeaveDayRow>>,
    captured_filters: Mutex<Option<AdminAttendanceExportFilters>>,
    captured_leave_filters: Mutex<Option<AdminAttendanceExportFilters>>,
}

#[async_trait::async_trait]
impl ExportAdminAttendanceRepository for RecordingAdminExportRepository {
    async fn list_subordinate_user_ids(
        &self,
        _manager_id: &str,
    ) -> Result<Vec<String>, ExportAdminAttendanceError> {
        Ok(self
            .subordinate_ids
            .lock()
            .expect("subordinate ids lock")
            .clone())
    }

    async fn list_admin_attendance_export(
        &self,
        filters: AdminAttendanceExportFilters,
    ) -> Result<Vec<AdminAttendanceExportRow>, ExportAdminAttendanceError> {
        *self.captured_filters.lock().expect("captured filters lock") = Some(filters);
        Ok(self.rows.lock().expect("rows lock").clone())
    }

    async fn list_admin_leave_days(
        &self,
        filters: AdminAttendanceExportFilters,
    ) -> Result<Vec<AdminLeaveDayRow>, ExportAdminAttendanceError> {
        *self
            .captured_leave_filters
            .lock()
            .expect("captured leave filters lock") = Some(filters);
        Ok(self.leave_days.lock().expect("leave days lock").clone())
    }
}

fn date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 6, 13).expect("date")
}

fn time(hour: u32) -> NaiveDateTime {
    date().and_hms_opt(hour, 0, 0).expect("time")
}

const EMPLOYEE_USER_ID: &str = "11111111-1111-1111-1111-111111111111";

fn export_row() -> AdminAttendanceExportRow {
    AdminAttendanceExportRow {
        user_id: EMPLOYEE_USER_ID.to_string(),
        username: "employee".to_string(),
        full_name_encrypted: "encrypted-name".to_string(),
        date: date(),
        clock_in_time: Some(time(9)),
        clock_out_time: Some(time(18)),
        total_work_hours: Some(8.0),
        status: "present".to_string(),
        leave_type: None,
    }
}

fn leave_day(day: u32, leave_type: &str) -> AdminLeaveDayRow {
    AdminLeaveDayRow {
        user_id: EMPLOYEE_USER_ID.to_string(),
        username: "employee".to_string(),
        full_name_encrypted: "encrypted-name".to_string(),
        date: NaiveDate::from_ymd_opt(2026, 6, day).expect("date"),
        leave_type: leave_type.to_string(),
    }
}

fn query() -> ExportAdminAttendanceQuery {
    ExportAdminAttendanceQuery {
        requester_id: "manager-1".to_string(),
        requester_is_manager: true,
        requester_is_system_admin: false,
        username: Some("employee".to_string()),
        from: Some(date()),
        to: Some(date()),
    }
}

#[tokio::test]
async fn rejects_non_admin_requester() {
    let use_case = ExportAdminAttendance::new(RecordingAdminExportRepository::default());

    let result = use_case
        .execute(ExportAdminAttendanceQuery {
            requester_is_manager: false,
            requester_is_system_admin: false,
            ..query()
        })
        .await;

    assert!(matches!(result, Err(ExportAdminAttendanceError::Forbidden)));
}

#[tokio::test]
async fn system_admin_export_is_unscoped_and_unmasked() {
    let repository = RecordingAdminExportRepository::default();
    *repository.rows.lock().expect("rows lock") = vec![export_row()];
    let use_case = ExportAdminAttendance::new(repository);

    let export = use_case
        .execute(ExportAdminAttendanceQuery {
            requester_is_manager: true,
            requester_is_system_admin: true,
            ..query()
        })
        .await
        .expect("export succeeds");

    assert!(!export.pii_masked);
    assert_eq!(export.rows.len(), 1);
}

#[tokio::test]
async fn manager_export_is_scoped_to_subordinates_and_masked() {
    let repository = RecordingAdminExportRepository::default();
    *repository
        .subordinate_ids
        .lock()
        .expect("subordinate ids lock") = vec!["user-1".to_string(), "user-2".to_string()];
    let use_case = ExportAdminAttendance::new(repository);

    let export = use_case.execute(query()).await.expect("export succeeds");
    let filters = use_case
        .repository()
        .captured_filters
        .lock()
        .expect("captured filters lock")
        .clone()
        .expect("filters captured");

    assert!(export.pii_masked);
    assert_eq!(
        filters.allowed_user_ids,
        Some(vec!["user-1".to_string(), "user-2".to_string()])
    );
    assert_eq!(filters.username.as_deref(), Some("employee"));

    let leave_filters = use_case
        .repository()
        .captured_leave_filters
        .lock()
        .expect("captured leave filters lock")
        .clone()
        .expect("leave filters captured");
    assert_eq!(leave_filters, filters);
}

#[tokio::test]
async fn export_includes_leave_type_column_and_derived_leave_rows() {
    let repository = RecordingAdminExportRepository::default();
    *repository.rows.lock().expect("rows lock") = vec![export_row()];
    *repository.leave_days.lock().expect("leave days lock") = vec![leave_day(15, "annual")];
    let use_case = ExportAdminAttendance::new(repository);

    let export = use_case
        .execute(ExportAdminAttendanceQuery {
            requester_is_system_admin: true,
            ..query()
        })
        .await
        .expect("export succeeds");

    assert_eq!(export.rows.len(), 2);
    assert_eq!(
        export.rows[0].date,
        NaiveDate::from_ymd_opt(2026, 6, 15).expect("date")
    );
    assert_eq!(export.rows[0].status, ON_LEAVE_STATUS);
    assert_eq!(export.rows[0].leave_type.as_deref(), Some("annual"));
    assert_eq!(export.rows[0].clock_in_time, None);
    assert_eq!(export.rows[0].clock_out_time, None);
    assert_eq!(export.rows[0].total_work_hours, None);
    assert_eq!(export.rows[1].date, date());
    assert_eq!(export.rows[1].status, "present");
    assert_eq!(export.rows[1].leave_type, None);
}

#[tokio::test]
async fn export_attaches_leave_type_to_punched_day_without_replacing_actuals() {
    let repository = RecordingAdminExportRepository::default();
    *repository.rows.lock().expect("rows lock") = vec![export_row()];
    *repository.leave_days.lock().expect("leave days lock") = vec![leave_day(13, "sick")];
    let use_case = ExportAdminAttendance::new(repository);

    let export = use_case
        .execute(ExportAdminAttendanceQuery {
            requester_is_system_admin: true,
            ..query()
        })
        .await
        .expect("export succeeds");

    assert_eq!(export.rows.len(), 1);
    assert_eq!(export.rows[0].date, date());
    assert_eq!(export.rows[0].status, "present");
    assert_eq!(export.rows[0].clock_in_time, Some(time(9)));
    assert_eq!(export.rows[0].clock_out_time, Some(time(18)));
    assert_eq!(export.rows[0].total_work_hours, Some(8.0));
    assert_eq!(export.rows[0].leave_type.as_deref(), Some("sick"));
}

#[tokio::test]
async fn export_does_not_merge_leave_across_different_user_ids_sharing_a_username() {
    // Regression test for M-9: the merge key must be (user_id, date), not
    // (username, date). A stale/duplicate username must not cause a leave
    // row belonging to a different user to be attached to this row.
    let repository = RecordingAdminExportRepository::default();
    *repository.rows.lock().expect("rows lock") = vec![export_row()];
    *repository.leave_days.lock().expect("leave days lock") = vec![AdminLeaveDayRow {
        user_id: "22222222-2222-2222-2222-222222222222".to_string(),
        username: "employee".to_string(),
        full_name_encrypted: "encrypted-name-2".to_string(),
        date: date(),
        leave_type: "annual".to_string(),
    }];
    let use_case = ExportAdminAttendance::new(repository);

    let export = use_case
        .execute(ExportAdminAttendanceQuery {
            requester_is_system_admin: true,
            ..query()
        })
        .await
        .expect("export succeeds");

    // The two rows share a date and username but belong to different
    // user_ids, so they must remain two distinct rows rather than being
    // merged into one.
    assert_eq!(export.rows.len(), 2);
    let attendance_row = export
        .rows
        .iter()
        .find(|row| row.user_id == EMPLOYEE_USER_ID)
        .expect("attendance row present");
    assert_eq!(attendance_row.status, "present");
    assert_eq!(attendance_row.leave_type, None);
    let leave_row = export
        .rows
        .iter()
        .find(|row| row.user_id == "22222222-2222-2222-2222-222222222222")
        .expect("leave row present");
    assert_eq!(leave_row.status, ON_LEAVE_STATUS);
    assert_eq!(leave_row.leave_type.as_deref(), Some("annual"));
}

#[tokio::test]
async fn rejects_date_range_exceeding_max_span() {
    let use_case = ExportAdminAttendance::new(RecordingAdminExportRepository::default());
    let from = date();
    let to = from + chrono::Duration::days(400);

    let result = use_case
        .execute(ExportAdminAttendanceQuery {
            requester_is_system_admin: true,
            from: Some(from),
            to: Some(to),
            ..query()
        })
        .await;

    assert!(matches!(
        result,
        Err(ExportAdminAttendanceError::DateRangeTooLarge { max_days: 366 })
    ));
}
