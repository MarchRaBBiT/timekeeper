use axum::{extract::Query, Extension};
use chrono::{Duration, NaiveDate};
use sqlx::PgPool;
use timekeeper_backend::{
    handlers::admin::{list_holidays, AdminHolidayListQuery},
    models::user::UserRole,
};
use uuid::Uuid;

mod support;
use support::{seed_user, seed_weekly_holiday, test_config};

#[sqlx::test(migrations = "./migrations")]
async fn admin_holiday_list_returns_all_kinds(pool: PgPool) {
    let config = test_config();
    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();

    sqlx::query(
        "INSERT INTO holidays (id, holiday_date, name, description, created_at, updated_at) \
         VALUES ($1,$2,$3,$4,NOW(),NOW())",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(date)
    .bind("New Year")
    .bind(Some("default".to_string()))
    .execute(&pool)
    .await
    .expect("insert holiday");

    seed_weekly_holiday(&pool, date).await;

    sqlx::query(
        "INSERT INTO holiday_exceptions \
         (id, user_id, exception_date, override, reason, created_by, created_at, updated_at) \
         VALUES ($1,$2,$3,$4,$5,$6,NOW(),NOW())",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&admin.id)
    .bind(date.succ_opt().unwrap())
    .bind(true)
    .bind(Some("override".to_string()))
    .bind("tester")
    .execute(&pool)
    .await
    .expect("insert exception");

    let query = AdminHolidayListQuery {
        page: Some(1),
        per_page: Some(10),
        r#type: None,
        from: None,
        to: None,
    };

    let response = list_holidays(
        axum::extract::State((pool.clone(), config.clone())),
        Extension(admin.clone()),
        Query(query),
    )
    .await
    .expect("list holidays ok");

    let payload = response.0;
    assert_eq!(payload.total, 3);
    assert_eq!(payload.items.len(), 3);
    assert!(payload.items.iter().any(|item| item.kind == "public"));
    assert!(payload.items.iter().any(|item| item.kind == "weekly"));
    assert!(payload.items.iter().any(|item| item.kind == "exception"));
}

#[sqlx::test(migrations = "./migrations")]
async fn admin_holiday_list_respects_filters(pool: PgPool) {
    let config = test_config();
    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let from = NaiveDate::from_ymd_opt(2025, 5, 1).unwrap();

    for idx in 0..3 {
        sqlx::query(
            "INSERT INTO holidays (id, holiday_date, name, description, created_at, updated_at) \
             VALUES ($1,$2,$3,NULL,NOW(),NOW())",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(from.succ_opt().unwrap() + Duration::days(idx))
        .bind(format!("Holiday {}", idx + 1))
        .execute(&pool)
        .await
        .expect("insert holiday");
    }

    let query = AdminHolidayListQuery {
        page: Some(2),
        per_page: Some(1),
        r#type: Some("public".into()),
        from: Some(from),
        to: None,
    };

    let response = list_holidays(
        axum::extract::State((pool.clone(), config.clone())),
        Extension(admin.clone()),
        Query(query),
    )
    .await
    .expect("paged holidays ok");

    let payload = response.0;
    assert_eq!(payload.total, 3);
    assert_eq!(payload.items.len(), 1);
    assert_eq!(payload.page, 2);
    assert_eq!(payload.per_page, 1);
    assert!(payload.items[0].kind == "public");
}
