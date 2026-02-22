use crate::models::holiday::{AdminHolidayKind, AdminHolidayListItem};
use crate::repositories::holiday::{HolidayRepository, HolidayRepositoryTrait};
use crate::repositories::repository::Repository;
use crate::repositories::weekly_holiday::WeeklyHolidayRepository;
use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::str::FromStr;
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

use crate::{
    error::AppError,
    models::{
        holiday::{
            CreateHolidayPayload, CreateWeeklyHolidayPayload, Holiday, HolidayResponse,
            WeeklyHoliday, WeeklyHolidayResponse,
        },
        user::User,
    },
    state::AppState,
    utils::time,
};

use super::common::parse_optional_date;

const DEFAULT_PAGE: i64 = 1;
const DEFAULT_PER_PAGE: i64 = 25;
const MAX_PER_PAGE: i64 = 100;
const MAX_PAGE: i64 = 1_000;

pub async fn list_holidays(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(q): Query<AdminHolidayListQuery>,
) -> Result<Json<AdminHolidayListResponse>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    let AdminHolidayQueryParams {
        page,
        per_page,
        kind,
        from,
        to,
    } = validate_admin_holiday_query(q)?;
    let offset = (page - 1) * per_page;

    let repo = HolidayRepository::new();
    let (items, total) = repo
        .list_paginated_admin(state.read_pool(), kind, from, to, per_page, offset)
        .await?;

    Ok(Json(AdminHolidayListResponse {
        page,
        per_page,
        total,
        items,
    }))
}

pub async fn create_holiday(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<CreateHolidayPayload>,
) -> Result<Json<HolidayResponse>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

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

    let holiday = Holiday::new(
        holiday_date,
        trimmed_name.to_string(),
        normalized_description,
    );

    let repo = HolidayRepository::new();
    let created = repo.create_unique(&state.write_pool, &holiday).await?;
    Ok(Json(HolidayResponse::from(created)))
}

pub async fn delete_holiday(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(holiday_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    let id = crate::types::HolidayId::from_str(&holiday_id)
        .map_err(|_| AppError::BadRequest("Invalid holiday ID".into()))?;

    let repo = HolidayRepository::new();
    repo.delete(&state.write_pool, id).await?;

    Ok(Json(json!({"message":"Holiday deleted","id": holiday_id})))
}

pub async fn list_weekly_holidays(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<WeeklyHolidayResponse>>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    let repo = WeeklyHolidayRepository::new();
    let holidays = repo.find_all(state.read_pool()).await?;

    Ok(Json(
        holidays
            .into_iter()
            .map(WeeklyHolidayResponse::from)
            .collect(),
    ))
}

pub async fn create_weekly_holiday(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<CreateWeeklyHolidayPayload>,
) -> Result<Json<WeeklyHolidayResponse>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    payload.validate()?;

    if let Some(ends_on) = payload.ends_on {
        if ends_on < payload.starts_on {
            return Err(AppError::BadRequest(
                "End date must be on or after the start date".into(),
            ));
        }
    }

    let today = time::today_local(&state.config.time_zone);
    let tomorrow = today + Duration::days(1);
    if !user.is_system_admin() && payload.starts_on < tomorrow {
        return Err(AppError::BadRequest(
            "Start date must be at least tomorrow".into(),
        ));
    }

    let weekly = WeeklyHoliday::new(payload.weekday, payload.starts_on, payload.ends_on, user.id);

    let repo = WeeklyHolidayRepository::new();
    repo.create(&state.write_pool, &weekly).await?;

    Ok(Json(WeeklyHolidayResponse::from(weekly)))
}

pub async fn delete_weekly_holiday(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    let id = crate::types::WeeklyHolidayId::from_str(&id)
        .map_err(|_| AppError::BadRequest("Invalid weekly holiday ID".into()))?;

    let repo = WeeklyHolidayRepository::new();
    repo.delete(&state.write_pool, id).await?;

    Ok(Json(json!({"message":"Weekly holiday deleted","id": id})))
}

// ============================================================================
// Internal helpers and types
// ============================================================================

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct AdminHolidayListQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    #[serde(rename = "type")]
    pub r#type: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
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

pub fn validate_admin_holiday_query(
    q: AdminHolidayListQuery,
) -> Result<AdminHolidayQueryParams, AppError> {
    let page = q.page.unwrap_or(DEFAULT_PAGE).clamp(1, MAX_PAGE);
    let per_page = q
        .per_page
        .unwrap_or(DEFAULT_PER_PAGE)
        .clamp(1, MAX_PER_PAGE);

    let kind =
        parse_type_filter(q.r#type.as_deref()).map_err(|e| AppError::BadRequest(e.into()))?;
    let from =
        parse_optional_date(q.from.as_deref()).map_err(|e| AppError::BadRequest(e.into()))?;
    let to = parse_optional_date(q.to.as_deref()).map_err(|e| AppError::BadRequest(e.into()))?;

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
    use chrono::NaiveDate;

    #[test]
    fn test_admin_holiday_list_query_default_values() {
        let query = AdminHolidayListQuery {
            page: None,
            per_page: None,
            r#type: None,
            from: None,
            to: None,
        };
        assert!(query.page.is_none());
        assert!(query.per_page.is_none());
        assert!(query.r#type.is_none());
        assert!(query.from.is_none());
        assert!(query.to.is_none());
    }

    #[test]
    fn test_admin_holiday_list_query_with_values() {
        let query = AdminHolidayListQuery {
            page: Some(2),
            per_page: Some(50),
            r#type: Some("public".to_string()),
            from: Some("2024-01-01".to_string()),
            to: Some("2024-12-31".to_string()),
        };
        assert_eq!(query.page, Some(2));
        assert_eq!(query.per_page, Some(50));
        assert_eq!(query.r#type, Some("public".to_string()));
        assert_eq!(query.from, Some("2024-01-01".to_string()));
        assert_eq!(query.to, Some("2024-12-31".to_string()));
    }

    #[test]
    fn test_admin_holiday_list_response_structure() {
        use crate::models::holiday::AdminHolidayKind;
        use crate::models::holiday::AdminHolidayListItem;
        use chrono::Utc;

        let item = AdminHolidayListItem {
            id: "test-id".to_string(),
            kind: AdminHolidayKind::Public,
            applies_from: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            applies_to: None,
            date: Some(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
            weekday: None,
            starts_on: None,
            ends_on: None,
            name: Some("New Year's Day".to_string()),
            description: Some("Public holiday".to_string()),
            user_id: None,
            reason: None,
            created_by: None,
            created_at: Utc::now(),
            is_override: None,
        };

        let response = AdminHolidayListResponse {
            page: 1,
            per_page: 25,
            total: 100,
            items: vec![item],
        };

        assert_eq!(response.page, 1);
        assert_eq!(response.per_page, 25);
        assert_eq!(response.total, 100);
        assert_eq!(response.items.len(), 1);
    }

    #[test]
    fn test_admin_holiday_query_params_structure() {
        use crate::models::holiday::AdminHolidayKind;

        let params = AdminHolidayQueryParams {
            page: 1,
            per_page: 25,
            kind: Some(AdminHolidayKind::Public),
            from: Some(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
            to: Some(NaiveDate::from_ymd_opt(2024, 12, 31).unwrap()),
        };

        assert_eq!(params.page, 1);
        assert_eq!(params.per_page, 25);
        assert_eq!(params.kind, Some(AdminHolidayKind::Public));
        assert!(params.from.is_some());
        assert!(params.to.is_some());
    }

    #[test]
    fn test_validate_admin_holiday_query_with_defaults() {
        let query = AdminHolidayListQuery {
            page: None,
            per_page: None,
            r#type: None,
            from: None,
            to: None,
        };

        let result = validate_admin_holiday_query(query);
        assert!(result.is_ok());

        let params = result.unwrap();
        assert_eq!(params.page, DEFAULT_PAGE);
        assert_eq!(params.per_page, DEFAULT_PER_PAGE);
        assert!(params.kind.is_none());
        assert!(params.from.is_none());
        assert!(params.to.is_none());
    }

    #[test]
    fn test_validate_admin_holiday_query_with_valid_values() {
        let query = AdminHolidayListQuery {
            page: Some(2),
            per_page: Some(50),
            r#type: Some("public".to_string()),
            from: Some("2024-01-01".to_string()),
            to: Some("2024-12-31".to_string()),
        };

        let result = validate_admin_holiday_query(query);
        assert!(result.is_ok());

        let params = result.unwrap();
        assert_eq!(params.page, 2);
        assert_eq!(params.per_page, 50);
        assert_eq!(params.kind, Some(AdminHolidayKind::Public));
        assert!(params.from.is_some());
        assert!(params.to.is_some());
    }

    #[test]
    fn test_validate_admin_holiday_query_rejects_invalid_date_range() {
        let query = AdminHolidayListQuery {
            page: None,
            per_page: None,
            r#type: None,
            from: Some("2024-12-31".to_string()),
            to: Some("2024-01-01".to_string()),
        };

        let result = validate_admin_holiday_query(query);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::BadRequest(_)));
    }

    #[test]
    fn test_parse_type_filter_with_public() {
        let result = parse_type_filter(Some("public"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(AdminHolidayKind::Public));
    }

    #[test]
    fn test_parse_type_filter_with_weekly() {
        let result = parse_type_filter(Some("weekly"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(AdminHolidayKind::Weekly));
    }

    #[test]
    fn test_parse_type_filter_with_exception() {
        let result = parse_type_filter(Some("exception"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(AdminHolidayKind::Exception));
    }

    #[test]
    fn test_parse_type_filter_with_all() {
        let result = parse_type_filter(Some("all"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_parse_type_filter_with_none() {
        let result = parse_type_filter(None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_parse_type_filter_rejects_invalid_type() {
        let result = parse_type_filter(Some("invalid"));
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "`type` must be one of public, weekly, exception, all"
        );
    }

    #[test]
    fn test_parse_type_filter_case_insensitive() {
        let result = parse_type_filter(Some("PUBLIC"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(AdminHolidayKind::Public));
    }

    #[test]
    fn test_validate_admin_holiday_query_clamps_page() {
        let query = AdminHolidayListQuery {
            page: Some(10000),
            per_page: None,
            r#type: None,
            from: None,
            to: None,
        };

        let result = validate_admin_holiday_query(query);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().page, MAX_PAGE);
    }

    #[test]
    fn test_validate_admin_holiday_query_clamps_per_page() {
        let query = AdminHolidayListQuery {
            page: None,
            per_page: Some(200),
            r#type: None,
            from: None,
            to: None,
        };

        let result = validate_admin_holiday_query(query);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().per_page, MAX_PER_PAGE);
    }
}
