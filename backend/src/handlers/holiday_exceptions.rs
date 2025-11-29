use std::sync::Arc;

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::NaiveDate;
use serde_json::{json, Value};
use sqlx::PgPool;
use tracing::error;

use crate::{
    config::Config,
    models::{
        holiday_exception::{CreateHolidayExceptionPayload, HolidayExceptionResponse},
        user::User,
    },
    services::holiday_exception::{HolidayExceptionError, HolidayExceptionService},
};

#[derive(Debug, serde::Deserialize)]
pub struct HolidayExceptionQuery {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

pub async fn create_holiday_exception(
    State((_pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Extension(service): Extension<Arc<HolidayExceptionService>>,
    Path(target_user_id): Path<String>,
    Json(payload): Json<CreateHolidayExceptionPayload>,
) -> Result<(StatusCode, Json<HolidayExceptionResponse>), (StatusCode, Json<Value>)> {
    ensure_admin_or_system(&user)?;

    let created = service
        .create_workday_override(&target_user_id, payload, &user.id)
        .await
        .map_err(map_exception_error)?;

    Ok((
        StatusCode::CREATED,
        Json(HolidayExceptionResponse::from(created)),
    ))
}

pub async fn list_holiday_exceptions(
    State((_pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Extension(service): Extension<Arc<HolidayExceptionService>>,
    Path(target_user_id): Path<String>,
    Query(query): Query<HolidayExceptionQuery>,
) -> Result<Json<Vec<HolidayExceptionResponse>>, (StatusCode, Json<Value>)> {
    ensure_admin_or_system(&user)?;

    let exceptions = service
        .list_for_user(&target_user_id, query.from, query.to)
        .await
        .map_err(map_exception_error)?;

    let response = exceptions
        .into_iter()
        .map(HolidayExceptionResponse::from)
        .collect();

    Ok(Json(response))
}

pub async fn delete_holiday_exception(
    State((_pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Extension(service): Extension<Arc<HolidayExceptionService>>,
    Path((target_user_id, id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    ensure_admin_or_system(&user)?;

    service
        .delete_for_user(&id, &target_user_id)
        .await
        .map_err(map_exception_error)?;

    Ok(StatusCode::NO_CONTENT)
}

pub fn map_exception_error(error: HolidayExceptionError) -> (StatusCode, Json<Value>) {
    match error {
        HolidayExceptionError::Conflict => (
            StatusCode::CONFLICT,
            Json(json!({"error":"Holiday exception already exists for this date"})),
        ),
        HolidayExceptionError::NotFound => (
            StatusCode::NOT_FOUND,
            Json(json!({"error":"Holiday exception not found"})),
        ),
        HolidayExceptionError::UserNotFound => (
            StatusCode::NOT_FOUND,
            Json(json!({"error":"User not found"})),
        ),
        HolidayExceptionError::Database(err) => {
            error!(error = ?err, "Holiday exception database error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error":"Database error"})),
            )
        }
    }
}

fn ensure_admin_or_system(user: &User) -> Result<(), (StatusCode, Json<Value>)> {
    if user.is_admin() || user.is_system_admin() {
        Ok(())
    } else {
        Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_exception_error_handles_user_not_found() {
        let (status, Json(body)) = map_exception_error(HolidayExceptionError::UserNotFound);
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.get("error"), Some(&json!("User not found")));
    }
}
