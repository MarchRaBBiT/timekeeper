use axum::{extract::Query, Extension};
use chrono::NaiveDate;
use timekeeper_backend::{
    handlers::admin::{list_holidays, AdminHolidayKind, AdminHolidayListQuery},
    models::user::UserRole,
};

mod support;
use support::{
    seed_holiday_exception, seed_public_holiday, seed_user, seed_weekly_holiday, setup_test_pool,
    test_config,
};

#[tokio::test]
async fn admin_holiday_list_filters_by_type() {
    let Some(pool) = setup_test_pool().await else {
        eprintln!("Skipping admin_holiday_list_filters_by_type: database unavailable");
        return;
    };
    let config = test_config();
    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let date = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();

    seed_public_holiday(&pool, date, "New Year").await;
    seed_weekly_holiday(&pool, date).await;
    seed_holiday_exception(&pool, &admin.id, date, true, "Override").await;

    let query = AdminHolidayListQuery {
        page: Some(1),
        per_page: Some(20),
        r#type: Some("public".into()),
        from: None,
        to: None,
    };

    let response = list_holidays(
        axum::extract::State((pool.clone(), config)),
        Extension(admin),
        Query(query),
    )
    .await
    .expect("admin list ok")
    .0;

    assert_eq!(response.total, 1);
    assert_eq!(response.items.len(), 1);
    assert_eq!(response.items[0].kind, AdminHolidayKind::Public);
    assert_eq!(response.items[0].date, Some(date));
}

#[tokio::test]
async fn admin_holiday_list_supports_pagination_and_range() {
    let Some(pool) = setup_test_pool().await else {
        eprintln!(
            "Skipping admin_holiday_list_supports_pagination_and_range: database unavailable"
        );
        return;
    };
    let config = test_config();
    let admin = seed_user(&pool, UserRole::Admin, false).await;

    let dates = [
        NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
        NaiveDate::from_ymd_opt(2025, 3, 5).unwrap(),
        NaiveDate::from_ymd_opt(2025, 3, 10).unwrap(),
    ];
    for (idx, date) in dates.iter().enumerate() {
        seed_public_holiday(&pool, *date, &format!("Holiday {}", idx)).await;
    }

    let query = AdminHolidayListQuery {
        page: Some(2),
        per_page: Some(1),
        r#type: Some("all".into()),
        from: Some("2025-03-01".into()),
        to: Some("2025-03-06".into()),
    };

    let response = list_holidays(
        axum::extract::State((pool.clone(), config)),
        Extension(admin),
        Query(query),
    )
    .await
    .expect("admin list ok")
    .0;

    assert_eq!(response.total, 2);
    assert_eq!(response.page, 2);
    assert_eq!(response.per_page, 1);
    // page 2 with per_page 1 over 2 items results in 1 item on last page
    assert_eq!(response.items.len(), 1);
    assert!(response
        .items
        .iter()
        .all(|item| item.kind == AdminHolidayKind::Public));
    assert!(response.items.iter().all(|item| item.applies_from
        >= NaiveDate::from_ymd_opt(2025, 3, 1).unwrap()
        && item.applies_from <= NaiveDate::from_ymd_opt(2025, 3, 6).unwrap()));
}
