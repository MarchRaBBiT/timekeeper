use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use timekeeper_contract::holiday_work::{
    HolidayWorkDecision, HolidayWorkListQuery, HolidayWorkRequestResponse, SubmitHolidayWorkRequest,
};
use uuid::Uuid;
use validator::Validate;

use crate::{
    error::AppError,
    models::user::User,
    repositories::holiday_work::{HolidayWorkRepository, HolidayWorkRepositoryError},
    state::AppState,
};

pub async fn submit(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<SubmitHolidayWorkRequest>,
) -> Result<Json<HolidayWorkRequestResponse>, AppError> {
    payload.validate()?;
    let response = HolidayWorkRepository::new(state.write_pool)
        .submit(&user.id.to_string(), payload)
        .await
        .map_err(map_error)?;
    Ok(Json(response))
}

pub async fn list_mine(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(query): Query<HolidayWorkListQuery>,
) -> Result<Json<Vec<HolidayWorkRequestResponse>>, AppError> {
    let response = HolidayWorkRepository::new(state.read_pool().clone())
        .list_for_user(&user.id.to_string(), query.status)
        .await
        .map_err(map_error)?;
    Ok(Json(response))
}

pub async fn cancel(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<Uuid>,
) -> Result<Json<HolidayWorkRequestResponse>, AppError> {
    let response = HolidayWorkRepository::new(state.write_pool)
        .cancel(id, &user.id.to_string())
        .await
        .map_err(map_error)?;
    Ok(Json(response))
}

pub async fn list_pending(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<HolidayWorkRequestResponse>>, AppError> {
    let response = HolidayWorkRepository::new(state.read_pool().clone())
        .list_pending_in_scope(&user.id.to_string(), user.is_system_admin())
        .await
        .map_err(map_error)?;
    Ok(Json(response))
}

pub async fn approve(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<Uuid>,
    Json(payload): Json<HolidayWorkDecision>,
) -> Result<Json<HolidayWorkRequestResponse>, AppError> {
    payload.validate()?;
    let repository = HolidayWorkRepository::new(state.write_pool.clone());
    let response = repository
        .approve(
            id,
            &user.id.to_string(),
            user.is_system_admin(),
            &payload.comment,
        )
        .await
        .map_err(map_error)?;
    Ok(Json(response))
}

pub async fn reject(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<Uuid>,
    Json(payload): Json<HolidayWorkDecision>,
) -> Result<Json<HolidayWorkRequestResponse>, AppError> {
    payload.validate()?;
    let repository = HolidayWorkRepository::new(state.write_pool.clone());
    let response = repository
        .reject(
            id,
            &user.id.to_string(),
            user.is_system_admin(),
            &payload.comment,
        )
        .await
        .map_err(map_error)?;
    Ok(Json(response))
}

fn map_error(error: HolidayWorkRepositoryError) -> AppError {
    match error {
        HolidayWorkRepositoryError::NotFound => {
            AppError::NotFound("Holiday work request not found".into())
        }
        HolidayWorkRepositoryError::NotPending => {
            AppError::Conflict("Holiday work request is not pending".into())
        }
        HolidayWorkRepositoryError::WorkdayNotResolved => {
            AppError::Conflict("Both workdays must be resolved before approval".into())
        }
        HolidayWorkRepositoryError::InvalidDayKinds => AppError::BadRequest(
            "Origin must be a non-working day and substitute must be a scheduled workday".into(),
        ),
        HolidayWorkRepositoryError::Locked => {
            AppError::Conflict("One or both workdays are locked".into())
        }
        HolidayWorkRepositoryError::CompensatoryAlreadyConsumed => AppError::Conflict(
            "Approved compensatory leave cannot be cancelled after consumption".into(),
        ),
        HolidayWorkRepositoryError::Unauthorized => {
            AppError::Forbidden("Actor cannot decide this holiday work request".into())
        }
        HolidayWorkRepositoryError::Domain(error) => AppError::BadRequest(error.to_string()),
        HolidayWorkRepositoryError::Repository(message) => {
            AppError::InternalServerError(anyhow::anyhow!(message))
        }
    }
}
