use sqlx::postgres::PgPoolOptions;
use timekeeper_app::attendance::{
    CancelAttendanceCorrectionRepository, CreateAttendanceCorrectionRepository,
    DecideAttendanceCorrectionRepository, ListAdminAttendanceCorrectionRequestsRepository,
    UpdateAttendanceCorrectionRepository,
};
use timekeeper_infra_postgres::attendance_correction::AttendanceCorrectionRepository;

fn assert_create_repository<T: CreateAttendanceCorrectionRepository>() {}
fn assert_update_repository<T: UpdateAttendanceCorrectionRepository>() {}
fn assert_cancel_repository<T: CancelAttendanceCorrectionRepository>() {}
fn assert_decision_repository<T: DecideAttendanceCorrectionRepository>() {}
fn assert_admin_read_repository<T: ListAdminAttendanceCorrectionRequestsRepository>() {}

#[test]
fn attendance_correction_repository_implements_app_ports() {
    assert_create_repository::<AttendanceCorrectionRepository>();
    assert_update_repository::<AttendanceCorrectionRepository>();
    assert_cancel_repository::<AttendanceCorrectionRepository>();
    assert_decision_repository::<AttendanceCorrectionRepository>();
    assert_admin_read_repository::<AttendanceCorrectionRepository>();
}

#[tokio::test]
async fn attendance_correction_repository_owns_pg_pool() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://timekeeper:test@127.0.0.1:5432/timekeeper")
        .expect("lazy pool");
    let repository = AttendanceCorrectionRepository::new(pool.clone());

    assert!(!repository.pool().is_closed());
}
