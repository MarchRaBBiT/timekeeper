use timekeeper_contract::{
    admin_attendance_report::{
        AdminAttendanceReportItem, AdminAttendanceReportResponse, ReportClassification,
    },
    work_schedules::OvertimeMonitorStatus,
};

#[test]
fn report_preserves_unresolved_classification_as_tagged_status() {
    let response = AdminAttendanceReportResponse {
        year: 2026,
        month: 7,
        page: 1,
        per_page: 25,
        total: 1,
        items: vec![AdminAttendanceReportItem {
            user_id: "u1".into(),
            user_name: "Alice".into(),
            department_id: None,
            department_name: None,
            classification: ReportClassification::UnresolvedDays,
            late_count: 0,
            early_leave_count: 0,
            absent_count: 0,
            anomaly_count: 0,
            overtime_status: OvertimeMonitorStatus::Warning,
        }],
    };
    let value = serde_json::to_value(response).expect("serialize report");
    assert_eq!(
        value["items"][0]["classification"]["status"],
        "unresolved_days"
    );
    assert!(value["items"][0]["classification"]
        .get("actual_minutes")
        .is_none());
    assert_eq!(value["items"][0]["overtime_status"], "warning");
}
