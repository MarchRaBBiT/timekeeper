use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    routing::get,
    Extension, Router,
};
use chrono::{NaiveDate, NaiveDateTime};
use serde_json::Value;
use timekeeper_backend::{
    handlers::admin::export_payroll,
    models::user::UserRole,
    repositories::work_schedule::{close_monthly_closing_workflow, transition_monthly_closing},
    state::AppState,
};
use timekeeper_contract::work_schedules::MonthlyClosingStatus;
use tower::ServiceExt;
use uuid::Uuid;

#[path = "support/mod.rs"]
mod support;
use support::integration_guard;

#[tokio::test]
async fn system_admin_export_returns_closed_snapshot_and_per_user_failures() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");
    let admin = support::seed_user(&pool, UserRole::Manager, true).await;
    let closed = support::seed_user(&pool, UserRole::Employee, false).await;
    let open = support::seed_user(&pool, UserRole::Employee, false).await;
    let workflow_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO monthly_closing_workflows (id,user_id,year,month,status,closed_by,closed_at)
         VALUES ($1,$2,2026,7,'closed',$3,NOW())",
    )
    .bind(workflow_id)
    .bind(closed.id.to_string())
    .bind(admin.id.to_string())
    .execute(&pool)
    .await
    .expect("closed workflow");
    sqlx::query(
        "INSERT INTO monthly_closing_workflows (id,user_id,year,month,status)
         VALUES ($1,$2,2026,7,'open')",
    )
    .bind(Uuid::new_v4())
    .bind(open.id.to_string())
    .execute(&pool)
    .await
    .expect("open workflow");
    sqlx::query(
        "INSERT INTO payroll_export_snapshots
         (id,workflow_id,user_id,year,month,revision,worked_minutes,scheduled_minutes,
          statutory_within_minutes,statutory_excess_minutes,legal_holiday_minutes,night_minutes,
          absent_days,paid_leave_days,
          paid_leave_half_days,paid_leave_minutes,holiday_work_minutes,
          substitute_holiday_days,compensatory_leave_minutes,created_by)
         VALUES ($1,$2,$3,2026,7,1,9600,9000,300,300,480,60,0,1,1,60,480,1,240,$4)",
    )
    .bind(Uuid::new_v4())
    .bind(workflow_id)
    .bind(closed.id.to_string())
    .bind(admin.id.to_string())
    .execute(&pool)
    .await
    .expect("snapshot");

    let app = Router::new()
        .route("/api/admin/payroll-export", get(export_payroll))
        .layer(Extension(admin))
        .with_state(AppState::new(
            pool.clone(),
            None,
            None,
            None,
            support::test_config(),
        ));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/admin/payroll-export?year=2026&month=7")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("body");
    let json: Value = serde_json::from_slice(&body).expect("json");
    assert!(json["csv"].as_str().expect("csv").starts_with('\u{feff}'));
    assert!(json["exported"]
        .as_array()
        .expect("exported")
        .iter()
        .any(|item| item["user_id"] == closed.id.to_string()));
    assert!(
        json["failed"]
            .as_array()
            .expect("failed")
            .iter()
            .any(|item| item["user_id"] == open.id.to_string()
                && item["code"] == "monthly_not_closed")
    );
}

#[tokio::test]
async fn close_appends_snapshot_reopen_blocks_and_reclose_increments_revision() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");
    let admin = support::seed_user(&pool, UserRole::Manager, true).await;
    let employee = support::seed_user(&pool, UserRole::Employee, false).await;
    let (schedule_id, version_id) =
        support::seed_work_schedule_for_user(&pool, employee.id, "non_working").await;
    for day in 1..=3 {
        sqlx::query(
            "INSERT INTO resolved_workdays
             (id,user_id,work_date,work_schedule_id,work_schedule_version_id,source,source_id,
              day_kind,timezone,workday_boundary,expected_work_minutes,resolved_at)
             VALUES ($1,$2,$3,$4,$5,'user',$6,$7,'Asia/Tokyo','05:00',$8,NOW())",
        )
        .bind(Uuid::new_v4())
        .bind(employee.id.to_string())
        .bind(NaiveDate::from_ymd_opt(2026, 8, day).expect("date"))
        .bind(schedule_id)
        .bind(version_id)
        .bind(Uuid::new_v4())
        .bind(if day == 2 {
            "scheduled_non_working_day"
        } else {
            "scheduled_workday"
        })
        .bind(if day == 2 { 0 } else { 480 })
        .execute(&pool)
        .await
        .expect("resolved day");
    }
    let dt = |day, hour| {
        NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2026, 8, day).expect("date"),
            chrono::NaiveTime::from_hms_opt(hour, 0, 0).expect("time"),
        )
    };
    let saturday = support::seed_attendance(
        &pool,
        employee.id,
        dt(1, 9).date(),
        Some(dt(1, 9)),
        Some(dt(1, 19)),
    )
    .await;
    support::seed_break_record(&pool, saturday.id, dt(1, 12), Some(dt(1, 13))).await;
    support::seed_attendance(
        &pool,
        employee.id,
        dt(2, 9).date(),
        Some(dt(2, 9)),
        Some(dt(2, 17)),
    )
    .await;
    sqlx::query(
        "INSERT INTO leave_requests
         (id,user_id,leave_type,start_date,end_date,status,approved_by,approved_at,acquisition_unit)
         VALUES ($1,$2,'annual','2026-07-31','2026-08-01','approved',$3,NOW(),'day'),
                ($4,$2,'annual','2026-08-03','2026-08-03','approved',$3,NOW(),'half_am')",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(employee.id.to_string())
    .bind(admin.id.to_string())
    .bind(Uuid::new_v4().to_string())
    .execute(&pool)
    .await
    .expect("day and half leave");
    sqlx::query(
        "INSERT INTO leave_requests
         (id,user_id,leave_type,start_date,end_date,status,approved_by,approved_at,
          acquisition_unit,start_time,end_time,requested_minutes)
         VALUES ($1,$2,'annual','2026-08-03','2026-08-03','approved',$3,NOW(),
                 'hour','14:00','15:00',60)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(employee.id.to_string())
    .bind(admin.id.to_string())
    .execute(&pool)
    .await
    .expect("hour leave");
    sqlx::query(
        "INSERT INTO holiday_work_requests
         (id,user_id,work_date,benefit,substitute_date,status,reason,decided_by,decided_at)
         VALUES ($1,$2,'2026-08-02','substitution','2026-08-10','approved',
                 'holiday operation',$3,NOW())",
    )
    .bind(Uuid::new_v4())
    .bind(employee.id.to_string())
    .bind(admin.id.to_string())
    .execute(&pool)
    .await
    .expect("substitution");
    sqlx::query(
        "INSERT INTO monthly_closing_workflows
         (id,user_id,year,month,status,self_confirmed_by,self_confirmed_at,
          approved_by,approved_at)
         VALUES ($1,$2,2026,8,'approved',$2,NOW(),$3,NOW())",
    )
    .bind(Uuid::new_v4())
    .bind(employee.id.to_string())
    .bind(admin.id.to_string())
    .execute(&pool)
    .await
    .expect("approved workflow");

    close_monthly_closing_workflow(
        &pool,
        &employee.id.to_string(),
        2026,
        8,
        &admin.id.to_string(),
        None,
    )
    .await
    .expect("first close");
    let revision: i32 = sqlx::query_scalar(
        "SELECT revision FROM payroll_export_snapshots
         WHERE user_id=$1 AND year=2026 AND month=8",
    )
    .bind(employee.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("revision");
    assert_eq!(revision, 1);
    let frozen: (i64, i64, i64, i64, i64, i64, i32, i32, i64, i64, i32) = sqlx::query_as(
        "SELECT worked_minutes,scheduled_minutes,statutory_within_minutes,
                    statutory_excess_minutes,legal_holiday_minutes,night_minutes,
                    paid_leave_days,paid_leave_half_days,paid_leave_minutes,
                    holiday_work_minutes,substitute_holiday_days
             FROM payroll_export_snapshots
             WHERE user_id=$1 AND year=2026 AND month=8 AND revision=1",
    )
    .bind(employee.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("frozen values");
    assert_eq!(frozen, (1020, 480, 0, 60, 480, 0, 1, 1, 60, 480, 1));
    let mutation = sqlx::query(
        "UPDATE payroll_export_snapshots SET worked_minutes = 1
         WHERE user_id=$1 AND year=2026 AND month=8 AND revision=1",
    )
    .bind(employee.id.to_string())
    .execute(&pool)
    .await;
    assert!(
        mutation.is_err(),
        "snapshot rows must be database-immutable"
    );

    transition_monthly_closing(
        &pool,
        &employee.id.to_string(),
        2026,
        8,
        MonthlyClosingStatus::Reopened,
        &admin.id.to_string(),
        Some("correction"),
    )
    .await
    .expect("reopen");
    let reopened = timekeeper_backend::repositories::payroll_export::export_payroll(&pool, 2026, 8)
        .await
        .expect("reopened export response");
    assert!(reopened
        .failed
        .iter()
        .any(|item| item.user_id == employee.id.to_string()));

    close_monthly_closing_workflow(
        &pool,
        &employee.id.to_string(),
        2026,
        8,
        &admin.id.to_string(),
        None,
    )
    .await
    .expect("reclose");
    let revisions: Vec<i32> = sqlx::query_scalar(
        "SELECT revision FROM payroll_export_snapshots
         WHERE user_id=$1 AND year=2026 AND month=8 ORDER BY revision",
    )
    .bind(employee.id.to_string())
    .fetch_all(&pool)
    .await
    .expect("revisions");
    assert_eq!(revisions, vec![1, 2]);
}
