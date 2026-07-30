use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::get,
    Extension, Router,
};
use chrono::{NaiveDate, NaiveDateTime};
use serde_json::Value;
use sqlx::PgPool;
use timekeeper_app::work_schedules::{ResolveWorkday, ResolveWorkdayCommand};
use timekeeper_backend::{
    handlers::admin::attendance_report::get_admin_attendance_report,
    models::user::{User, UserRole},
    state::AppState,
};
use timekeeper_infra_postgres::work_schedules::WorkdayResolverPostgresRepository;
use tower::ServiceExt;
use uuid::Uuid;

mod support;
use support::{
    integration_guard, seed_attendance, seed_user, seed_work_schedule_for_user, test_config,
    test_pool,
};

fn router(pool: PgPool, actor: User) -> Router {
    Router::new()
        .route(
            "/api/admin/attendance-report",
            get(get_admin_attendance_report),
        )
        .layer(Extension(actor))
        .with_state(AppState::new(pool, None, None, None, test_config()))
}

async fn get_json(app: Router, uri: &str) -> (StatusCode, Value) {
    let response = app
        .oneshot(
            Request::builder()
                .uri(uri)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

async fn department(pool: &PgPool, name: &str) -> String {
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO departments (id, name) VALUES ($1, $2)")
        .bind(&id)
        .bind(name)
        .execute(pool)
        .await
        .expect("department");
    id
}

async fn assign(pool: &PgPool, user: &User, department_id: &str) {
    sqlx::query("UPDATE users SET department_id = $1 WHERE id = $2")
        .bind(department_id)
        .bind(user.id.to_string())
        .execute(pool)
        .await
        .expect("assign department");
}

async fn manage(pool: &PgPool, manager: &User, department_id: &str) {
    sqlx::query("INSERT INTO department_managers (department_id, user_id) VALUES ($1, $2)")
        .bind(department_id)
        .bind(manager.id.to_string())
        .execute(pool)
        .await
        .expect("assign manager");
}

fn at(day: u32, hour: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 7, day)
        .expect("date")
        .and_hms_opt(hour, 0, 0)
        .expect("time")
}

#[tokio::test]
async fn system_admin_and_manager_scope_pagination_and_severity_are_enforced() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let manager = seed_user(&pool, UserRole::Manager, false).await;
    let exceeded = seed_user(&pool, UserRole::Employee, false).await;
    let ok = seed_user(&pool, UserRole::Employee, false).await;
    let outside = seed_user(&pool, UserRole::Employee, false).await;
    let managed_department = department(&pool, "Managed report").await;
    let outside_department = department(&pool, "Outside report").await;
    assign(&pool, &exceeded, &managed_department).await;
    assign(&pool, &ok, &managed_department).await;
    assign(&pool, &outside, &outside_department).await;
    manage(&pool, &manager, &managed_department).await;
    seed_work_schedule_for_user(&pool, exceeded.id, "non_working").await;
    let resolver_repository = WorkdayResolverPostgresRepository::new(pool.clone());
    let resolver = ResolveWorkday::new(
        resolver_repository.clone(),
        resolver_repository.clone(),
        resolver_repository,
    );
    for day in 1..=4 {
        resolver
            .execute(ResolveWorkdayCommand {
                user_id: exceeded.id.to_string(),
                work_date: NaiveDate::from_ymd_opt(2026, 7, day).expect("date"),
                resolved_at: chrono::Utc::now(),
            })
            .await
            .expect("resolve workday");
        seed_attendance(
            &pool,
            exceeded.id,
            NaiveDate::from_ymd_opt(2026, 7, day).expect("date"),
            Some(at(day, 0)),
            Some(at(day, 20)),
        )
        .await;
    }

    let (status, report) = get_json(
        router(pool.clone(), admin),
        "/api/admin/attendance-report?year=2026&month=7&page=1&per_page=100",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let items = report["items"].as_array().expect("items");
    assert!(items
        .iter()
        .any(|item| item["user_id"] == outside.id.to_string()));
    let exceeded_index = items
        .iter()
        .position(|item| item["user_id"] == exceeded.id.to_string())
        .expect("exceeded item");
    let ok_index = items
        .iter()
        .position(|item| item["user_id"] == ok.id.to_string())
        .expect("ok item");
    assert_eq!(items[exceeded_index]["overtime_status"], "exceeded");
    assert!(
        exceeded_index < ok_index,
        "severity sort must precede pagination"
    );

    let (page_status, page) = get_json(
        router(pool.clone(), manager.clone()),
        "/api/admin/attendance-report?year=2026&month=7&page=1&per_page=1",
    )
    .await;
    assert_eq!(page_status, StatusCode::OK);
    assert_eq!(page["total"], 2);
    assert_eq!(page["items"].as_array().expect("page items").len(), 1);
    assert!(page["items"][0]["user_id"] != outside.id.to_string());

    let (forbidden, _) = get_json(
        router(pool, manager),
        &format!(
            "/api/admin/attendance-report?year=2026&month=7&department_id={outside_department}"
        ),
    )
    .await;
    assert_eq!(forbidden, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn invalid_year_month_and_department_are_bad_requests() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    for uri in [
        "/api/admin/attendance-report?year=1899&month=7",
        "/api/admin/attendance-report?year=2026&month=13",
        "/api/admin/attendance-report?year=2026&month=7&department_id=not-a-uuid",
    ] {
        let (status, _) = get_json(router(pool.clone(), admin.clone()), uri).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}");
    }
}
