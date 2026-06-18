use sqlx::postgres::PgPoolOptions;
use timekeeper_app::attendance::{
    AttendanceRepository, AttendanceStatusReadRepository, BreakEndRepository, ClockOutRepository,
    ExportAdminAttendanceRepository, GetBreaksByAttendanceRepository, ListActiveBreaksRepository,
    ListAttendancePageRepository, ListUserAttendanceRepository, StartBreakRepository,
    UpsertAttendanceRepository,
};
use timekeeper_infra_postgres::attendance::AttendanceWorkflowRepository;

fn assert_attendance_repository<T: AttendanceRepository>() {}
fn assert_attendance_status_read_repository<T: AttendanceStatusReadRepository>() {}
fn assert_clock_out_repository<T: ClockOutRepository>() {}
fn assert_start_break_repository<T: StartBreakRepository>() {}
fn assert_break_end_repository<T: BreakEndRepository>() {}
fn assert_get_breaks_repository<T: GetBreaksByAttendanceRepository>() {}
fn assert_list_active_breaks_repository<T: ListActiveBreaksRepository>() {}
fn assert_list_attendance_page_repository<T: ListAttendancePageRepository>() {}
fn assert_list_user_attendance_repository<T: ListUserAttendanceRepository>() {}
fn assert_upsert_attendance_repository<T: UpsertAttendanceRepository>() {}
fn assert_export_admin_attendance_repository<T: ExportAdminAttendanceRepository>() {}

#[test]
fn attendance_workflow_repository_implements_app_ports() {
    assert_attendance_repository::<AttendanceWorkflowRepository>();
    assert_attendance_status_read_repository::<AttendanceWorkflowRepository>();
    assert_clock_out_repository::<AttendanceWorkflowRepository>();
    assert_start_break_repository::<AttendanceWorkflowRepository>();
    assert_break_end_repository::<AttendanceWorkflowRepository>();
    assert_get_breaks_repository::<AttendanceWorkflowRepository>();
    assert_list_active_breaks_repository::<AttendanceWorkflowRepository>();
    assert_list_attendance_page_repository::<AttendanceWorkflowRepository>();
    assert_list_user_attendance_repository::<AttendanceWorkflowRepository>();
    assert_upsert_attendance_repository::<AttendanceWorkflowRepository>();
    assert_export_admin_attendance_repository::<AttendanceWorkflowRepository>();
}

#[tokio::test]
async fn attendance_workflow_repository_owns_pg_pool() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://timekeeper:test@127.0.0.1:5432/timekeeper")
        .expect("lazy pool");
    let repository = AttendanceWorkflowRepository::new(pool.clone());

    assert!(!repository.pool().is_closed());
}
