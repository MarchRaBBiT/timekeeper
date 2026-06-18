use std::sync::Mutex;

use chrono::{NaiveDate, NaiveDateTime};
use timekeeper_app::attendance::{
    AdminAttendanceExportFilters, AdminAttendanceExportRow, ExportAdminAttendance,
    ExportAdminAttendanceError, ExportAdminAttendanceQuery, ExportAdminAttendanceRepository,
};

#[derive(Default)]
struct RecordingAdminExportRepository {
    subordinate_ids: Mutex<Vec<String>>,
    rows: Mutex<Vec<AdminAttendanceExportRow>>,
    captured_filters: Mutex<Option<AdminAttendanceExportFilters>>,
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
}

fn date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 6, 13).expect("date")
}

fn time(hour: u32) -> NaiveDateTime {
    date().and_hms_opt(hour, 0, 0).expect("time")
}

fn export_row() -> AdminAttendanceExportRow {
    AdminAttendanceExportRow {
        username: "employee".to_string(),
        full_name_encrypted: "encrypted-name".to_string(),
        date: date(),
        clock_in_time: Some(time(9)),
        clock_out_time: Some(time(18)),
        total_work_hours: Some(8.0),
        status: "present".to_string(),
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
}
