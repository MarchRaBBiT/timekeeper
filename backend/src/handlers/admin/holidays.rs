use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};

use crate::{
    error::AppError,
    holiday::application::admin as application,
    models::{
        holiday::{
            AdminHolidayListItem, CreateHolidayPayload, CreateWeeklyHolidayPayload,
            HolidayResponse, WeeklyHolidayResponse,
        },
        user::User,
    },
    state::AppState,
};

pub async fn list_holidays(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(q): Query<AdminHolidayListQuery>,
) -> Result<Json<AdminHolidayListResponse>, AppError> {
    let response = application::list_holidays(
        state.read_pool(),
        &user,
        application::AdminHolidayListInput {
            page: q.page,
            per_page: q.per_page,
            type_filter: q.r#type,
            from: q.from,
            to: q.to,
        },
    )
    .await?;

    Ok(Json(AdminHolidayListResponse::from(response)))
}

pub async fn create_holiday(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<CreateHolidayPayload>,
) -> Result<Json<HolidayResponse>, AppError> {
    Ok(Json(
        application::create_holiday(&state.write_pool, &user, payload).await?,
    ))
}

pub async fn delete_holiday(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(holiday_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    Ok(Json(
        application::delete_holiday(&state.write_pool, &user, &holiday_id).await?,
    ))
}

pub async fn list_weekly_holidays(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<WeeklyHolidayResponse>>, AppError> {
    Ok(Json(
        application::list_weekly_holidays(state.read_pool(), &user).await?,
    ))
}

pub async fn create_weekly_holiday(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<CreateWeeklyHolidayPayload>,
) -> Result<Json<WeeklyHolidayResponse>, AppError> {
    Ok(Json(
        application::create_weekly_holiday(&state.write_pool, &state.config, &user, payload)
            .await?,
    ))
}

pub async fn delete_weekly_holiday(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    Ok(Json(
        application::delete_weekly_holiday(&state.write_pool, &user, &id).await?,
    ))
}

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

impl From<application::AdminHolidayListResponse> for AdminHolidayListResponse {
    fn from(value: application::AdminHolidayListResponse) -> Self {
        Self {
            page: value.page,
            per_page: value.per_page,
            total: value.total,
            items: value.items,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::holiday::AdminHolidayKind;
    use chrono::{NaiveDate, Utc};

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
        let item = AdminHolidayListItem {
            id: "test-id".to_string(),
            kind: AdminHolidayKind::Public,
            applies_from: NaiveDate::from_ymd_opt(2024, 1, 1).expect("valid date"),
            applies_to: None,
            date: Some(NaiveDate::from_ymd_opt(2024, 1, 1).expect("valid date")),
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
    fn test_admin_holiday_list_response_from_application_response() {
        let response = application::AdminHolidayListResponse {
            page: 1,
            per_page: 25,
            total: 0,
            items: vec![],
        };

        let mapped = AdminHolidayListResponse::from(response);
        assert_eq!(mapped.page, 1);
        assert_eq!(mapped.per_page, 25);
        assert_eq!(mapped.total, 0);
        assert!(mapped.items.is_empty());
    }
}
