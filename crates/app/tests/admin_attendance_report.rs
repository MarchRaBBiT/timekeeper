use async_trait::async_trait;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use timekeeper_app::admin_attendance_report::{
    AdminAttendanceReportCommand, AdminAttendanceReportError, AdminAttendanceReportRepository,
    GetAdminAttendanceReport,
};
use timekeeper_contract::{
    admin_attendance_report::{
        AdminAttendanceReportItem, AdminAttendanceReportQuery, ReportClassification,
    },
    work_schedules::OvertimeMonitorStatus,
};

#[derive(Clone)]
struct StubRepository(Vec<AdminAttendanceReportItem>);

#[async_trait]
impl AdminAttendanceReportRepository for StubRepository {
    async fn list_scoped_month(
        &self,
        _: &str,
        _: bool,
        _: i32,
        _: u32,
        department_id: Option<&str>,
    ) -> Result<Vec<AdminAttendanceReportItem>, AdminAttendanceReportError> {
        if department_id == Some("outside") {
            return Err(AdminAttendanceReportError::ForbiddenDepartment);
        }
        Ok(self.0.clone())
    }
}

#[derive(Clone)]
struct CountingBatchRepository {
    calls: Arc<AtomicUsize>,
    items: Vec<AdminAttendanceReportItem>,
}

#[async_trait]
impl AdminAttendanceReportRepository for CountingBatchRepository {
    async fn list_scoped_month(
        &self,
        _: &str,
        _: bool,
        _: i32,
        _: u32,
        _: Option<&str>,
    ) -> Result<Vec<AdminAttendanceReportItem>, AdminAttendanceReportError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.items.clone())
    }
}

fn item(id: &str, status: OvertimeMonitorStatus) -> AdminAttendanceReportItem {
    AdminAttendanceReportItem {
        user_id: id.into(),
        user_name: id.into(),
        department_id: Some("d1".into()),
        department_name: Some("Dept".into()),
        classification: ReportClassification::WorkRuleNotConfigured,
        late_count: 0,
        early_leave_count: 0,
        absent_count: 0,
        anomaly_count: 0,
        overtime_status: status,
    }
}

fn command(page: i64, per_page: i64) -> AdminAttendanceReportCommand {
    AdminAttendanceReportCommand {
        actor_user_id: "manager".into(),
        actor_is_system_admin: false,
        query: AdminAttendanceReportQuery {
            year: 2026,
            month: 7,
            department_id: None,
            page,
            per_page,
        },
    }
}

#[tokio::test]
async fn severity_sort_is_global_before_pagination_and_user_id_is_stable_tie_breaker() {
    let use_case = GetAdminAttendanceReport::new(StubRepository(vec![
        item("u3", OvertimeMonitorStatus::Ok),
        item("u2", OvertimeMonitorStatus::Exceeded),
        item("u1", OvertimeMonitorStatus::Exceeded),
        item("u4", OvertimeMonitorStatus::Warning),
    ]));
    let first = use_case.execute(command(1, 2)).await.expect("first page");
    let second = use_case.execute(command(2, 2)).await.expect("second page");
    assert_eq!(first.total, 4);
    assert_eq!(
        first
            .items
            .iter()
            .map(|item| item.user_id.as_str())
            .collect::<Vec<_>>(),
        ["u1", "u2"]
    );
    assert_eq!(
        second
            .items
            .iter()
            .map(|item| item.user_id.as_str())
            .collect::<Vec<_>>(),
        ["u4", "u3"]
    );
}

#[tokio::test]
async fn scope_failure_and_invalid_pagination_are_explicit() {
    let use_case = GetAdminAttendanceReport::new(StubRepository(Vec::new()));
    let mut outside = command(1, 25);
    outside.query.department_id = Some("outside".into());
    assert_eq!(
        use_case.execute(outside).await,
        Err(AdminAttendanceReportError::ForbiddenDepartment)
    );
    assert_eq!(
        use_case.execute(command(0, 25)).await,
        Err(AdminAttendanceReportError::InvalidPage)
    );
}

#[tokio::test]
async fn repository_batch_call_count_does_not_grow_with_user_count() {
    let calls = Arc::new(AtomicUsize::new(0));
    let items = (0..100)
        .map(|index| item(&format!("u{index:03}"), OvertimeMonitorStatus::Ok))
        .collect();
    let use_case = GetAdminAttendanceReport::new(CountingBatchRepository {
        calls: Arc::clone(&calls),
        items,
    });

    let response = use_case
        .execute(command(1, 100))
        .await
        .expect("batch report");

    assert_eq!(response.total, 100);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
