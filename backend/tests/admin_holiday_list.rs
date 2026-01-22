use axum::extract::{Extension, Query, State};
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use chrono_tz::UTC;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::str::FromStr;
#[cfg(feature = "test-utils")]
use std::sync::{Mutex, OnceLock};
#[cfg(feature = "test-utils")]
use std::time::Duration;
use timekeeper_backend::{
    config::Config,
    error::AppError,
    handlers::admin::holidays::{
        list_holidays, AdminHolidayListQuery, AdminHolidayListResponse,
    },
    models::{
        holiday::{AdminHolidayKind, AdminHolidayListItem},
        user::{User, UserRole},
    },
    state::AppState,
    utils::cookies::SameSite,
};

#[cfg(feature = "test-utils")]
fn integration_guard() -> std::sync::MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("lock integration guard")
}

fn build_item(
    id: &str,
    kind: AdminHolidayKind,
    date: NaiveDate,
    created_at: DateTime<Utc>,
) -> AdminHolidayListItem {
    AdminHolidayListItem {
        id: id.to_string(),
        kind,
        applies_from: date,
        applies_to: Some(date),
        date: Some(date),
        weekday: None,
        starts_on: None,
        ends_on: None,
        name: Some(id.to_string()),
        description: None,
        user_id: None,
        reason: None,
        created_by: None,
        created_at,
        is_override: None,
    }
}

fn mock_list_holidays(
    items: Vec<AdminHolidayListItem>,
    query: AdminHolidayListQuery,
) -> Result<AdminHolidayListResponse, String> {
    let (page, per_page) = (
        query.page.unwrap_or(1).clamp(1, 1_000),
        query.per_page.unwrap_or(25).clamp(1, 100),
    );

    let kind = match query.r#type.as_deref() {
        Some(value) if value.eq_ignore_ascii_case("all") => None,
        Some(value) => AdminHolidayKind::from_str(value)
            .map(Some)
            .map_err(|_| "`type` must be one of public, weekly, exception, all".to_string())?,
        None => None,
    };

    let parse_date = |s: &str| {
        DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.date_naive())
            .or_else(|_| NaiveDate::parse_from_str(s, "%Y-%m-%d"))
            .ok()
    };

    let from = query
        .from
        .as_deref()
        .map(|s| parse_date(s).ok_or("`from`/`to` must be a valid date (YYYY-MM-DD or RFC3339)"))
        .transpose()?;
    let to = query
        .to
        .as_deref()
        .map(|s| parse_date(s).ok_or("`from`/`to` must be a valid date (YYYY-MM-DD or RFC3339)"))
        .transpose()?;

    if let (Some(from), Some(to)) = (from, to) {
        if from > to {
            return Err("`from` must be before or equal to `to`".into());
        }
    }

    let mut filtered: Vec<AdminHolidayListItem> = items
        .into_iter()
        .filter(|item| kind.map(|k| item.kind == k).unwrap_or(true))
        .filter(|item| from.map(|f| item.applies_from >= f).unwrap_or(true))
        .filter(|item| to.map(|t| item.applies_from <= t).unwrap_or(true))
        .collect();

    filtered.sort_by(|a, b| {
        b.applies_from
            .cmp(&a.applies_from)
            .then_with(|| {
                fn kind_sort_key(kind: &AdminHolidayKind) -> &'static str {
                    match kind {
                        AdminHolidayKind::Exception => "exception",
                        AdminHolidayKind::Public => "public",
                        AdminHolidayKind::Weekly => "weekly",
                    }
                }
                kind_sort_key(&a.kind).cmp(kind_sort_key(&b.kind))
            })
            .then_with(|| b.created_at.cmp(&a.created_at))
    });

    let total = filtered.len() as i64;
    let offset = ((page - 1) * per_page) as usize;
    let items = filtered
        .into_iter()
        .skip(offset)
        .take(per_page as usize)
        .collect();

    Ok(AdminHolidayListResponse {
        page,
        per_page,
        total,
        items,
    })
}

fn test_admin() -> User {
    User::new(
        "admin".into(),
        "hash".into(),
        "Administrator".into(),
        "admin@example.com".into(),
        UserRole::Admin,
        true,
    )
}

fn dummy_config() -> Config {
    Config {
        database_url: "postgres://invalid:invalid@localhost:5432/invalid".into(),
        read_database_url: None,
        jwt_secret: "this_is_a_long_enough_jwt_secret_string_123".into(),
        jwt_expiration_hours: 1,
        refresh_token_expiration_days: 7,
        audit_log_retention_days: 1825,
        audit_log_retention_forever: false,
        consent_log_retention_days: 1825,
        consent_log_retention_forever: false,
        aws_region: "ap-northeast-1".into(),
        aws_kms_key_id: "alias/timekeeper-test".into(),
        aws_audit_log_bucket: "timekeeper-audit-logs".into(),
        aws_cloudtrail_enabled: true,
        cookie_secure: false,
        cookie_same_site: SameSite::Lax,
        cors_allow_origins: vec!["http://localhost:8000".into()],
        time_zone: UTC,
        mfa_issuer: "Timekeeper".into(),
        rate_limit_ip_max_requests: 15,
        rate_limit_ip_window_seconds: 900,
        rate_limit_user_max_requests: 20,
        rate_limit_user_window_seconds: 3600,
        redis_url: None,
        redis_pool_size: 10,
        redis_connect_timeout: 5,
        feature_redis_cache_enabled: true,
        feature_read_replica_enabled: true,
        password_min_length: 12,
        password_require_uppercase: true,
        password_require_lowercase: true,
        password_require_numbers: true,
        password_require_symbols: true,
        password_expiration_days: 90,
        password_history_count: 5,
    }
}

#[test]
fn admin_holiday_list_filters_by_type() {
    let base_date = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();
    let created_at = Utc.timestamp_millis_opt(0).single().unwrap();
    let items = vec![
        build_item("public", AdminHolidayKind::Public, base_date, created_at),
        build_item("weekly", AdminHolidayKind::Weekly, base_date, created_at),
        build_item(
            "exception",
            AdminHolidayKind::Exception,
            base_date,
            created_at,
        ),
    ];

    let query = AdminHolidayListQuery {
        page: Some(1),
        per_page: Some(20),
        r#type: Some("public".into()),
        from: None,
        to: None,
    };

    let response = mock_list_holidays(items, query).expect("filtering should succeed");

    assert_eq!(response.total, 1);
    assert_eq!(response.items.len(), 1);
    assert_eq!(response.items[0].kind, AdminHolidayKind::Public);
    assert_eq!(response.items[0].date, Some(base_date));
}

#[test]
fn admin_holiday_list_supports_pagination_and_range() {
    let dates = [
        NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
        NaiveDate::from_ymd_opt(2025, 3, 5).unwrap(),
        NaiveDate::from_ymd_opt(2025, 3, 10).unwrap(),
    ];
    let base_created = Utc.timestamp_millis_opt(10_000).single().unwrap();
    let items: Vec<_> = dates
        .iter()
        .enumerate()
        .map(|(idx, d)| {
            let created_at = base_created + chrono::Duration::milliseconds(idx as i64);
            build_item(
                &format!("Holiday {}", idx),
                AdminHolidayKind::Public,
                *d,
                created_at,
            )
        })
        .collect();

    let query = AdminHolidayListQuery {
        page: Some(2),
        per_page: Some(1),
        r#type: Some("all".into()),
        from: Some("2025-03-01".into()),
        to: Some("2025-03-06".into()),
    };

    let response = mock_list_holidays(items, query).expect("pagination should succeed");

    assert_eq!(response.total, 2);
    assert_eq!(response.page, 2);
    assert_eq!(response.per_page, 1);
    assert_eq!(response.items.len(), 1);
    assert!(response
        .items
        .iter()
        .all(|item| item.kind == AdminHolidayKind::Public));
    assert_eq!(
        response.items[0].applies_from,
        NaiveDate::from_ymd_opt(2025, 3, 1).unwrap()
    );
}

#[tokio::test]
async fn admin_holiday_list_rejects_unknown_type() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://invalid:invalid@localhost:5432/invalid")
        .expect("create lazy pool");

    let query = AdminHolidayListQuery {
        page: Some(1),
        per_page: Some(10),
        r#type: Some("unknown-kind".into()),
        from: None,
        to: None,
    };

    let result = list_holidays(
        State(AppState::new(pool, None, None, None, dummy_config())),
        Extension(test_admin()),
        Query(query),
    )
    .await;

    match result {
        Ok(_) => panic!("expected bad request for invalid type"),
        Err(AppError::BadRequest(msg)) => {
            assert_eq!(msg, "`type` must be one of public, weekly, exception, all");
        }
        Err(e) => panic!("unexpected error: {:?}", e),
    }
}

#[tokio::test]
async fn admin_holiday_list_rejects_invalid_date_format() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://invalid:invalid@localhost:5432/invalid")
        .expect("create lazy pool");

    let query = AdminHolidayListQuery {
        page: Some(1),
        per_page: Some(10),
        r#type: Some("public".into()),
        from: Some("not-a-date".into()),
        to: None,
    };

    let result = list_holidays(
        State(AppState::new(pool, None, None, None, dummy_config())),
        Extension(test_admin()),
        Query(query),
    )
    .await;

    match result {
        Ok(_) => panic!("expected bad request for invalid date"),
        Err(AppError::BadRequest(msg)) => {
            assert_eq!(
                msg,
                "`from`/`to` must be a valid date (YYYY-MM-DD or RFC3339)"
            );
        }
        Err(e) => panic!("unexpected error: {:?}", e),
    }
}

#[tokio::test]
async fn admin_holiday_list_rejects_from_after_to() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://invalid:invalid@localhost:5432/invalid")
        .expect("create lazy pool");

    let query = AdminHolidayListQuery {
        page: Some(1),
        per_page: Some(10),
        r#type: Some("public".into()),
        from: Some("2025-05-10".into()),
        to: Some("2025-05-01".into()),
    };

    let result = list_holidays(
        State(AppState::new(pool, None, None, None, dummy_config())),
        Extension(test_admin()),
        Query(query),
    )
    .await;

    match result {
        Ok(_) => panic!("expected bad request when from > to"),
        Err(AppError::BadRequest(msg)) => {
            assert_eq!(msg, "`from` must be before or equal to `to`");
        }
        Err(e) => panic!("unexpected error: {:?}", e),
    }
}

#[test]
fn admin_holiday_list_orders_by_date_kind_and_created_at() {
    let date_new = NaiveDate::from_ymd_opt(2025, 5, 10).unwrap();
    let date_old = NaiveDate::from_ymd_opt(2025, 5, 1).unwrap();
    let items = vec![
        build_item(
            "weekly-old-created",
            AdminHolidayKind::Weekly,
            date_new,
            Utc.timestamp_millis_opt(100).single().unwrap(),
        ),
        build_item(
            "weekly-new-created",
            AdminHolidayKind::Weekly,
            date_new,
            Utc.timestamp_millis_opt(500).single().unwrap(),
        ),
        build_item(
            "public",
            AdminHolidayKind::Public,
            date_new,
            Utc.timestamp_millis_opt(200).single().unwrap(),
        ),
        build_item(
            "exception",
            AdminHolidayKind::Exception,
            date_new,
            Utc.timestamp_millis_opt(300).single().unwrap(),
        ),
        build_item(
            "older",
            AdminHolidayKind::Public,
            date_old,
            Utc.timestamp_millis_opt(400).single().unwrap(),
        ),
    ];

    let response = mock_list_holidays(
        items,
        AdminHolidayListQuery {
            page: Some(1),
            per_page: Some(10),
            r#type: Some("all".into()),
            from: None,
            to: None,
        },
    )
    .expect("ordering should succeed");

    let ids: Vec<_> = response.items.iter().map(|i| i.id.as_str()).collect();
    assert_eq!(
        ids,
        &[
            "exception",          // same date, kind asc => exception first
            "public",             // then public
            "weekly-new-created", // then weekly (newer created_at first)
            "weekly-old-created",
            "older" // older date last
        ]
    );
}

#[test]
fn admin_holiday_list_clamps_page_and_per_page() {
    let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let created_at = Utc.timestamp_millis_opt(0).single().unwrap();
    let items = (0..3)
        .map(|i| {
            build_item(
                &format!("id-{i}"),
                AdminHolidayKind::Public,
                date,
                created_at,
            )
        })
        .collect();

    let response = mock_list_holidays(
        items,
        AdminHolidayListQuery {
            page: Some(0),
            per_page: Some(200),
            r#type: None,
            from: None,
            to: None,
        },
    )
    .expect("clamping should succeed");

    assert_eq!(response.page, 1);
    assert_eq!(response.per_page, 100);
    assert_eq!(response.total, 3);
}

#[allow(dead_code)]
async fn prepare_holiday_tables(pool: &PgPool) {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS holidays (
            id TEXT PRIMARY KEY,
            holiday_date DATE NOT NULL,
            name TEXT NOT NULL,
            description TEXT,
            created_at TIMESTAMPTZ NOT NULL,
            updated_at TIMESTAMPTZ NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await
    .expect("create holidays");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS weekly_holidays (
            id TEXT PRIMARY KEY,
            weekday SMALLINT NOT NULL,
            starts_on DATE NOT NULL,
            ends_on DATE,
            enforced_from DATE NOT NULL,
            enforced_to DATE,
            created_by TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL,
            updated_at TIMESTAMPTZ NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await
    .expect("create weekly_holidays");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS holiday_exceptions (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            exception_date DATE NOT NULL,
            override BOOLEAN NOT NULL,
            reason TEXT NOT NULL,
            created_by TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL,
            updated_at TIMESTAMPTZ NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await
    .expect("create holiday_exceptions");

    sqlx::query("TRUNCATE TABLE holiday_exceptions, weekly_holidays, holidays")
        .execute(pool)
        .await
        .expect("truncate tables");
}

#[allow(dead_code)]
async fn seed_integration_holidays(pool: &PgPool) {
    let created_base = Utc
        .timestamp_millis_opt(1_742_961_600_000)
        .single()
        .unwrap(); // 2025-03-10T00:00:00Z

    sqlx::query(
        "INSERT INTO holidays (id, holiday_date, name, description, created_at, updated_at) \
         VALUES ($1, $2, $3, NULL, $4, $4)",
    )
    .bind("public-310")
    .bind(NaiveDate::from_ymd_opt(2025, 3, 10).unwrap())
    .bind("Public Holiday")
    .bind(created_base + chrono::Duration::seconds(10))
    .execute(pool)
    .await
    .expect("insert public 310");

    sqlx::query(
        "INSERT INTO weekly_holidays (id, weekday, starts_on, ends_on, enforced_from, enforced_to, created_by, created_at, updated_at) \
         VALUES ($1, $2, $3, NULL, $4, NULL, 'tester', $5, $5)",
    )
    .bind("weekly-310")
    .bind(1_i16)
    .bind(NaiveDate::from_ymd_opt(2025, 3, 1).unwrap())
    .bind(NaiveDate::from_ymd_opt(2025, 3, 10).unwrap())
    .bind(created_base + chrono::Duration::seconds(20))
    .execute(pool)
    .await
    .expect("insert weekly 310");

    sqlx::query(
        "INSERT INTO holiday_exceptions (id, user_id, exception_date, override, reason, created_by, created_at, updated_at) \
         VALUES ($1, 'user', $2, true, 'exceptional', 'tester', $3, $3)",
    )
    .bind("exception-310")
    .bind(NaiveDate::from_ymd_opt(2025, 3, 10).unwrap())
    .bind(created_base + chrono::Duration::seconds(30))
    .execute(pool)
    .await
    .expect("insert exception 310");

    sqlx::query(
        "INSERT INTO holidays (id, holiday_date, name, description, created_at, updated_at) \
         VALUES ($1, $2, $3, NULL, $4, $4)",
    )
    .bind("public-305")
    .bind(NaiveDate::from_ymd_opt(2025, 3, 5).unwrap())
    .bind("Earlier Public")
    .bind(created_base - chrono::Duration::seconds(5 * 24 * 3600))
    .execute(pool)
    .await
    .expect("insert public 305");
}

#[cfg(feature = "test-utils")]
async fn connect_with_retry(database_url: &str) -> PgPool {
    let mut last_err = None;
    for _ in 0..5 {
        match PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(120))
            .connect(database_url)
            .await
        {
            Ok(pool) => return pool,
            Err(e) => {
                last_err = Some(e);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
    panic!("connect test database: {last_err:?}");
}

#[tokio::test]
#[cfg(feature = "test-utils")]
#[ignore = "requires Postgres setup; run with --features test-utils and enable when network/storage available"]
async fn admin_holiday_list_integration_success() {
    let _guard = integration_guard();
    let config = support::test_config();
    let pool = connect_with_retry(&config.database_url).await;
    prepare_holiday_tables(&pool).await;
    seed_integration_holidays(&pool).await;

    let query = AdminHolidayListQuery {
        page: Some(1),
        per_page: Some(2),
        r#type: Some("all".into()),
        from: Some("2025-03-05".into()),
        to: Some("2025-03-10".into()),
    };

    let Json(resp) = list_holidays(
        State((pool.clone(), config.clone())),
        Extension(test_admin()),
        Query(query),
    )
    .await
    .expect("list_holidays should succeed");

    assert_eq!(resp.total, 4);
    assert_eq!(resp.items.len(), 2);
    let ids: Vec<_> = resp.items.iter().map(|i| i.id.as_str()).collect();
    assert_eq!(ids, vec!["exception-310", "public-310"]);

    let query_page2 = AdminHolidayListQuery {
        page: Some(2),
        per_page: Some(2),
        r#type: Some("all".into()),
        from: Some("2025-03-05".into()),
        to: Some("2025-03-10".into()),
    };

    let Json(resp2) = list_holidays(
        State((pool, config)),
        Extension(test_admin()),
        Query(query_page2),
    )
    .await
    .expect("list_holidays page 2 should succeed");

    let ids2: Vec<_> = resp2.items.iter().map(|i| i.id.as_str()).collect();
    assert_eq!(ids2, vec!["weekly-310", "public-305"]);
}

#[tokio::test]
#[cfg(feature = "test-utils")]
#[ignore = "requires Postgres setup; run with --features test-utils and enable when network/storage available"]
async fn admin_holiday_list_integration_type_filtering() {
    let _guard = integration_guard();
    let config = support::test_config();
    let pool = connect_with_retry(&config.database_url).await;
    prepare_holiday_tables(&pool).await;
    seed_integration_holidays(&pool).await;

    // Public only
    let Json(public_resp) = list_holidays(
        State((pool.clone(), config.clone())),
        Extension(test_admin()),
        Query(AdminHolidayListQuery {
            page: Some(1),
            per_page: Some(10),
            r#type: Some("public".into()),
            from: None,
            to: None,
        }),
    )
    .await
    .expect("list_holidays public");
    let public_ids: Vec<_> = public_resp.items.iter().map(|i| i.id.as_str()).collect();
    assert_eq!(public_resp.total, 2);
    assert_eq!(public_ids, vec!["public-310", "public-305"]);

    // Weekly only
    let Json(weekly_resp) = list_holidays(
        State((pool.clone(), config.clone())),
        Extension(test_admin()),
        Query(AdminHolidayListQuery {
            page: Some(1),
            per_page: Some(10),
            r#type: Some("weekly".into()),
            from: None,
            to: None,
        }),
    )
    .await
    .expect("list_holidays weekly");
    let weekly_ids: Vec<_> = weekly_resp.items.iter().map(|i| i.id.as_str()).collect();
    assert_eq!(weekly_resp.total, 1);
    assert_eq!(weekly_ids, vec!["weekly-310"]);

    // Exception only
    let Json(exception_resp) = list_holidays(
        State((pool, config)),
        Extension(test_admin()),
        Query(AdminHolidayListQuery {
            page: Some(1),
            per_page: Some(10),
            r#type: Some("exception".into()),
            from: None,
            to: None,
        }),
    )
    .await
    .expect("list_holidays exception");
    let exception_ids: Vec<_> = exception_resp.items.iter().map(|i| i.id.as_str()).collect();
    assert_eq!(exception_resp.total, 1);
    assert_eq!(exception_ids, vec!["exception-310"]);
}

#[test]
fn validate_admin_holiday_query_accepts_valid_input() {
    let result = mock_list_holidays(
        vec![],
        AdminHolidayListQuery {
            page: Some(2),
            per_page: Some(50),
            r#type: Some("weekly".into()),
            from: Some("2025-01-01".into()),
            to: Some("2025-12-31".into()),
        },
    );
    assert!(result.is_ok(), "validation should succeed");
}

#[test]
fn validate_admin_holiday_query_rejects_invalid_type() {
    let err = mock_list_holidays(
        vec![],
        AdminHolidayListQuery {
            page: None,
            per_page: None,
            r#type: Some("unknown".into()),
            from: None,
            to: None,
        },
    )
    .expect_err("invalid type should fail");
    assert_eq!(err, "`type` must be one of public, weekly, exception, all");
}

#[test]
fn validate_admin_holiday_query_rejects_invalid_date_formats() {
    let err = mock_list_holidays(
        vec![],
        AdminHolidayListQuery {
            page: Some(1),
            per_page: Some(10),
            r#type: Some("public".into()),
            from: Some("2025/01/01".into()),
            to: None,
        },
    )
    .expect_err("invalid date format should fail");
    assert_eq!(
        err,
        "`from`/`to` must be a valid date (YYYY-MM-DD or RFC3339)"
    );
}

#[test]
fn validate_admin_holiday_query_rejects_from_after_to() {
    let err = mock_list_holidays(
        vec![],
        AdminHolidayListQuery {
            page: Some(1),
            per_page: Some(10),
            r#type: Some("public".into()),
            from: Some("2025-02-01".into()),
            to: Some("2025-01-01".into()),
        },
    )
    .expect_err("from > to should fail");
    assert_eq!(err, "`from` must be before or equal to `to`");
}

#[test]
fn validate_admin_holiday_query_clamps_page_and_per_page_boundaries() {
    let response = mock_list_holidays(
        vec![],
        AdminHolidayListQuery {
            page: Some(0),
            per_page: Some(200),
            r#type: None,
            from: None,
            to: None,
        },
    )
    .expect("clamping should succeed");

    assert_eq!(response.page, 1);
    assert_eq!(response.per_page, 100);
}
