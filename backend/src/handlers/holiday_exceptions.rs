use std::sync::Arc;

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::NaiveDate;

use crate::{
    error::AppError,
    holiday::application::exceptions as application,
    models::{
        holiday_exception::{CreateHolidayExceptionPayload, HolidayExceptionResponse},
        user::User,
    },
    services::holiday_exception::HolidayExceptionServiceTrait,
    state::AppState,
};

#[derive(Debug, serde::Deserialize)]
pub struct HolidayExceptionQuery {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

pub async fn create_holiday_exception(
    State(_state): State<AppState>,
    Extension(user): Extension<User>,
    Extension(service): Extension<Arc<dyn HolidayExceptionServiceTrait>>,
    Path(target_user_id): Path<String>,
    Json(payload): Json<CreateHolidayExceptionPayload>,
) -> Result<(StatusCode, Json<HolidayExceptionResponse>), AppError> {
    let created =
        application::create_holiday_exception(service.as_ref(), &user, &target_user_id, payload)
            .await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn list_holiday_exceptions(
    State(_state): State<AppState>,
    Extension(user): Extension<User>,
    Extension(service): Extension<Arc<dyn HolidayExceptionServiceTrait>>,
    Path(target_user_id): Path<String>,
    Query(query): Query<HolidayExceptionQuery>,
) -> Result<Json<Vec<HolidayExceptionResponse>>, AppError> {
    let response = application::list_holiday_exceptions(
        service.as_ref(),
        &user,
        &target_user_id,
        query.from,
        query.to,
    )
    .await?;

    Ok(Json(response))
}

pub async fn delete_holiday_exception(
    State(_state): State<AppState>,
    Extension(user): Extension<User>,
    Extension(service): Extension<Arc<dyn HolidayExceptionServiceTrait>>,
    Path((target_user_id, id)): Path<(String, String)>,
) -> Result<StatusCode, AppError> {
    application::delete_holiday_exception(service.as_ref(), &user, &target_user_id, &id).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn holiday_exception_query_deserializes_date_range() {
        let query = HolidayExceptionQuery {
            from: Some(NaiveDate::from_ymd_opt(2026, 3, 1).expect("valid from")),
            to: Some(NaiveDate::from_ymd_opt(2026, 3, 31).expect("valid to")),
        };

        assert_eq!(
            query.from,
            Some(NaiveDate::from_ymd_opt(2026, 3, 1).expect("valid from"))
        );
        assert_eq!(
            query.to,
            Some(NaiveDate::from_ymd_opt(2026, 3, 31).expect("valid to"))
        );
    }

    #[test]
    fn holiday_exception_query_allows_open_range() {
        let query = HolidayExceptionQuery {
            from: None,
            to: None,
        };

        assert!(query.from.is_none());
        assert!(query.to.is_none());
    }
}
