use std::str::FromStr;

use chrono::{Duration, NaiveDate};
use serde::Serialize;
use serde_json::{json, Value};

use crate::{
    admin::application::common::parse_optional_date,
    config::Config,
    error::AppError,
    models::{
        holiday::{
            AdminHolidayKind, AdminHolidayListItem, CreateHolidayPayload,
            CreateWeeklyHolidayPayload, Holiday, HolidayResponse, WeeklyHoliday,
            WeeklyHolidayResponse,
        },
        user::User,
    },
    repositories::{
        holiday::{HolidayRepository, HolidayRepositoryTrait},
        repository::Repository,
        weekly_holiday::WeeklyHolidayRepository,
    },
    utils::time,
};

const DEFAULT_PAGE: i64 = 1;
const DEFAULT_PER_PAGE: i64 = 25;
const MAX_PER_PAGE: i64 = 100;
const MAX_PAGE: i64 = 1_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminHolidayListInput {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub type_filter: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AdminHolidayListResponse {
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub items: Vec<AdminHolidayListItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminHolidayQueryParams {
    pub page: i64,
    pub per_page: i64,
    pub kind: Option<AdminHolidayKind>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

pub async fn list_holidays(
    read_pool: &sqlx::PgPool,
    user: &User,
    input: AdminHolidayListInput,
) -> Result<AdminHolidayListResponse, AppError> {
    ensure_admin(user)?;

    let AdminHolidayQueryParams {
        page,
        per_page,
        kind,
        from,
        to,
    } = validate_admin_holiday_query(input)?;
    let offset = (page - 1) * per_page;

    let repo = HolidayRepository::new();
    let (items, total) = repo
        .list_paginated_admin(read_pool, kind, from, to, per_page, offset)
        .await?;

    Ok(AdminHolidayListResponse {
        page,
        per_page,
        total,
        items,
    })
}

pub async fn create_holiday(
    write_pool: &sqlx::PgPool,
    user: &User,
    payload: CreateHolidayPayload,
) -> Result<HolidayResponse, AppError> {
    ensure_admin(user)?;

    let holiday = build_holiday(payload)?;
    let repo = HolidayRepository::new();
    let created = repo.create_unique(write_pool, &holiday).await?;
    Ok(HolidayResponse::from(created))
}

pub async fn delete_holiday(
    write_pool: &sqlx::PgPool,
    user: &User,
    holiday_id: &str,
) -> Result<Value, AppError> {
    ensure_admin(user)?;

    let id = crate::types::HolidayId::from_str(holiday_id)
        .map_err(|_| AppError::BadRequest("Invalid holiday ID".into()))?;

    let repo = HolidayRepository::new();
    repo.delete(write_pool, id).await?;

    Ok(json!({"message":"Holiday deleted","id": holiday_id}))
}

pub async fn list_weekly_holidays(
    read_pool: &sqlx::PgPool,
    user: &User,
) -> Result<Vec<WeeklyHolidayResponse>, AppError> {
    ensure_admin(user)?;

    let repo = WeeklyHolidayRepository::new();
    let holidays = repo.find_all(read_pool).await?;
    Ok(holidays
        .into_iter()
        .map(WeeklyHolidayResponse::from)
        .collect())
}

pub async fn create_weekly_holiday(
    write_pool: &sqlx::PgPool,
    config: &Config,
    user: &User,
    payload: CreateWeeklyHolidayPayload,
) -> Result<WeeklyHolidayResponse, AppError> {
    ensure_admin(user)?;
    validate_weekly_holiday_payload(&payload, config, user)?;

    let weekly = WeeklyHoliday::new(payload.weekday, payload.starts_on, payload.ends_on, user.id);
    let repo = WeeklyHolidayRepository::new();
    repo.create(write_pool, &weekly).await?;
    Ok(WeeklyHolidayResponse::from(weekly))
}

pub async fn delete_weekly_holiday(
    write_pool: &sqlx::PgPool,
    user: &User,
    id: &str,
) -> Result<Value, AppError> {
    ensure_admin(user)?;

    let id = crate::types::WeeklyHolidayId::from_str(id)
        .map_err(|_| AppError::BadRequest("Invalid weekly holiday ID".into()))?;

    let repo = WeeklyHolidayRepository::new();
    repo.delete(write_pool, id).await?;

    Ok(json!({"message":"Weekly holiday deleted","id": id}))
}

pub fn validate_admin_holiday_query(
    input: AdminHolidayListInput,
) -> Result<AdminHolidayQueryParams, AppError> {
    let page = input.page.unwrap_or(DEFAULT_PAGE).clamp(1, MAX_PAGE);
    let per_page = input
        .per_page
        .unwrap_or(DEFAULT_PER_PAGE)
        .clamp(1, MAX_PER_PAGE);

    let kind = parse_type_filter(input.type_filter.as_deref())
        .map_err(|e| AppError::BadRequest(e.into()))?;
    let from =
        parse_optional_date(input.from.as_deref()).map_err(|e| AppError::BadRequest(e.into()))?;
    let to =
        parse_optional_date(input.to.as_deref()).map_err(|e| AppError::BadRequest(e.into()))?;

    if let (Some(from), Some(to)) = (from, to) {
        if from > to {
            return Err(AppError::BadRequest(
                "`from` must be before or equal to `to`".into(),
            ));
        }
    }

    Ok(AdminHolidayQueryParams {
        page,
        per_page,
        kind,
        from,
        to,
    })
}

pub fn validate_weekly_holiday_payload(
    payload: &CreateWeeklyHolidayPayload,
    config: &Config,
    user: &User,
) -> Result<(), AppError> {
    if payload.weekday > 6 {
        return Err(AppError::BadRequest(
            "Weekday must be between 0 (Sun) and 6 (Sat). (Sun=0, Mon=1, ..., Sat=6)".into(),
        ));
    }

    if let Some(ends_on) = payload.ends_on {
        if ends_on < payload.starts_on {
            return Err(AppError::BadRequest(
                "End date must be on or after the start date".into(),
            ));
        }
    }

    let today = time::today_local(&config.time_zone);
    let tomorrow = today + Duration::days(1);
    if !user.is_system_admin() && payload.starts_on < tomorrow {
        return Err(AppError::BadRequest(
            "Start date must be at least tomorrow".into(),
        ));
    }

    Ok(())
}

pub fn build_holiday(payload: CreateHolidayPayload) -> Result<Holiday, AppError> {
    let CreateHolidayPayload {
        holiday_date,
        name,
        description,
    } = payload;

    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        return Err(AppError::BadRequest("Holiday name is required".into()));
    }

    let normalized_description = description.as_deref().map(str::trim).and_then(|d| {
        if d.is_empty() {
            None
        } else {
            Some(d.to_string())
        }
    });

    Ok(Holiday::new(
        holiday_date,
        trimmed_name.to_string(),
        normalized_description,
    ))
}

pub fn ensure_admin(user: &User) -> Result<(), AppError> {
    if user.is_admin() {
        Ok(())
    } else {
        Err(AppError::Forbidden("Forbidden".into()))
    }
}

fn parse_type_filter(raw: Option<&str>) -> Result<Option<AdminHolidayKind>, &'static str> {
    match raw {
        Some(value) if value.eq_ignore_ascii_case("all") => Ok(None),
        Some(value) => AdminHolidayKind::from_str(value)
            .map(Some)
            .map_err(|_| "`type` must be one of public, weekly, exception, all"),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::user::UserRole;
    use chrono_tz::UTC;

    fn sample_user(role: UserRole, is_system_admin: bool) -> User {
        let now = chrono::Utc::now();
        User {
            id: crate::types::UserId::new(),
            username: "user".to_string(),
            password_hash: "hash".to_string(),
            full_name: "User".to_string(),
            email: "user@example.com".to_string(),
            role,
            is_system_admin,
            mfa_secret: None,
            mfa_enabled_at: None,
            password_changed_at: now,
            failed_login_attempts: 0,
            locked_until: None,
            lock_reason: None,
            lockout_count: 0,
            created_at: now,
            updated_at: now,
        }
    }

    fn config() -> Config {
        Config {
            database_url: "postgres://test".to_string(),
            read_database_url: None,
            jwt_secret: "test-jwt-secret-32-chars-minimum!".to_string(),
            jwt_expiration_hours: 1,
            refresh_token_expiration_days: 7,
            max_concurrent_sessions: 3,
            audit_log_retention_days: 1825,
            audit_log_retention_forever: false,
            consent_log_retention_days: 1825,
            consent_log_retention_forever: false,
            aws_region: "ap-northeast-1".to_string(),
            aws_kms_key_id: "alias/timekeeper-test".to_string(),
            aws_audit_log_bucket: "timekeeper-audit-logs".to_string(),
            aws_cloudtrail_enabled: true,
            cookie_secure: false,
            cookie_same_site: crate::utils::cookies::SameSite::Lax,
            cors_allow_origins: vec!["*".to_string()],
            time_zone: UTC,
            mfa_issuer: "Timekeeper".to_string(),
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
            account_lockout_threshold: 5,
            account_lockout_duration_minutes: 15,
            account_lockout_backoff_enabled: true,
            account_lockout_max_duration_hours: 24,
            production_mode: false,
        }
    }

    #[test]
    fn validate_admin_holiday_query_with_defaults() {
        let params = validate_admin_holiday_query(AdminHolidayListInput {
            page: None,
            per_page: None,
            type_filter: None,
            from: None,
            to: None,
        })
        .expect("defaults should validate");

        assert_eq!(params.page, DEFAULT_PAGE);
        assert_eq!(params.per_page, DEFAULT_PER_PAGE);
        assert!(params.kind.is_none());
    }

    #[test]
    fn validate_admin_holiday_query_with_valid_values() {
        let params = validate_admin_holiday_query(AdminHolidayListInput {
            page: Some(2),
            per_page: Some(50),
            type_filter: Some("public".to_string()),
            from: Some("2024-01-01".to_string()),
            to: Some("2024-12-31".to_string()),
        })
        .expect("query should validate");

        assert_eq!(params.page, 2);
        assert_eq!(params.per_page, 50);
        assert_eq!(params.kind, Some(AdminHolidayKind::Public));
    }

    #[test]
    fn validate_admin_holiday_query_rejects_invalid_date_range() {
        let result = validate_admin_holiday_query(AdminHolidayListInput {
            page: None,
            per_page: None,
            type_filter: None,
            from: Some("2024-12-31".to_string()),
            to: Some("2024-01-01".to_string()),
        });

        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[test]
    fn build_holiday_trims_name_and_description() {
        let holiday = build_holiday(CreateHolidayPayload {
            holiday_date: NaiveDate::from_ymd_opt(2026, 5, 3).expect("valid date"),
            name: " Constitution Day ".to_string(),
            description: Some(" holiday ".to_string()),
        })
        .expect("payload should be valid");

        assert_eq!(holiday.name, "Constitution Day");
        assert_eq!(holiday.description.as_deref(), Some("holiday"));
    }

    #[test]
    fn build_holiday_rejects_blank_name() {
        let result = build_holiday(CreateHolidayPayload {
            holiday_date: NaiveDate::from_ymd_opt(2026, 5, 3).expect("valid date"),
            name: "   ".to_string(),
            description: None,
        });

        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[test]
    fn validate_weekly_holiday_payload_rejects_bad_weekday() {
        let result = validate_weekly_holiday_payload(
            &CreateWeeklyHolidayPayload {
                weekday: 7,
                starts_on: NaiveDate::from_ymd_opt(2026, 3, 11).expect("valid date"),
                ends_on: None,
            },
            &config(),
            &sample_user(UserRole::Admin, false),
        );

        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[test]
    fn validate_weekly_holiday_payload_rejects_end_before_start() {
        let result = validate_weekly_holiday_payload(
            &CreateWeeklyHolidayPayload {
                weekday: 1,
                starts_on: NaiveDate::from_ymd_opt(2026, 3, 11).expect("valid date"),
                ends_on: Some(NaiveDate::from_ymd_opt(2026, 3, 10).expect("valid date")),
            },
            &config(),
            &sample_user(UserRole::Admin, false),
        );

        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[test]
    fn validate_weekly_holiday_payload_allows_system_admin_today() {
        let today = time::today_local(&config().time_zone);
        let result = validate_weekly_holiday_payload(
            &CreateWeeklyHolidayPayload {
                weekday: 1,
                starts_on: today,
                ends_on: None,
            },
            &config(),
            &sample_user(UserRole::Employee, true),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn ensure_admin_rejects_regular_user() {
        let result = ensure_admin(&sample_user(UserRole::Employee, false));
        assert!(matches!(result, Err(AppError::Forbidden(_))));
    }
}
