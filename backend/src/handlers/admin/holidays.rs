use crate::models::holiday::{AdminHolidayKind, AdminHolidayListItem};
use crate::repositories::holiday::HolidayRepository;
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
