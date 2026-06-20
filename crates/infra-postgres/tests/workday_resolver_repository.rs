use sqlx::postgres::PgPoolOptions;
use timekeeper_app::work_schedules::{
    OrganizationHierarchy, WorkdayHolidayCalendar, WorkdayResolutionRepository,
};
use timekeeper_infra_postgres::work_schedules::WorkdayResolverPostgresRepository;

fn assert_resolution_repository<T: WorkdayResolutionRepository>() {}
fn assert_organization_hierarchy<T: OrganizationHierarchy>() {}
fn assert_holiday_calendar<T: WorkdayHolidayCalendar>() {}

#[test]
fn repository_implements_resolver_ports() {
    assert_resolution_repository::<WorkdayResolverPostgresRepository>();
    assert_organization_hierarchy::<WorkdayResolverPostgresRepository>();
    assert_holiday_calendar::<WorkdayResolverPostgresRepository>();
}

#[tokio::test]
async fn repository_owns_pg_pool() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://timekeeper:test@127.0.0.1:5432/timekeeper")
        .expect("lazy pool");
    let repository = WorkdayResolverPostgresRepository::new(pool.clone());

    assert!(!repository.pool().is_closed());
}
