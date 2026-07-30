use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    Json,
};
use sqlx::FromRow;
use timekeeper_contract::leave::{
    CreateLeaveTypeRequest, LeaveTypeResponse, LeaveUnit, UpdateLeaveTypeRequest,
};
use validator::Validate;

use crate::{error::AppError, models::user::User, state::AppState};

#[derive(Debug, FromRow)]
struct LeaveTypeRow {
    code: String,
    name: String,
    is_paid: bool,
    balance_tracked: bool,
    allowed_units: Vec<String>,
    is_active: bool,
}

pub async fn list_leave_types(
    Extension(state): Extension<AppState>,
    Extension(actor): Extension<User>,
) -> Result<Json<Vec<LeaveTypeResponse>>, AppError> {
    ensure_system_admin(&actor)?;
    let rows = sqlx::query_as::<_, LeaveTypeRow>(
        "SELECT code, name, is_paid, balance_tracked, allowed_units, is_active
         FROM leave_types ORDER BY code",
    )
    .fetch_all(state.read_pool())
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(row_to_response)
            .collect::<Result<_, _>>()?,
    ))
}

pub async fn create_leave_type(
    Extension(state): Extension<AppState>,
    Extension(actor): Extension<User>,
    Json(payload): Json<CreateLeaveTypeRequest>,
) -> Result<(StatusCode, Json<LeaveTypeResponse>), AppError> {
    ensure_system_admin(&actor)?;
    payload.validate()?;
    validate_code(&payload.code)?;
    let units = units_to_db(&payload.allowed_units);
    let row = sqlx::query_as::<_, LeaveTypeRow>(
        "INSERT INTO leave_types
            (code, name, is_paid, balance_tracked, allowed_units)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING code, name, is_paid, balance_tracked, allowed_units, is_active",
    )
    .bind(&payload.code)
    .bind(payload.name.trim())
    .bind(payload.is_paid)
    .bind(payload.balance_tracked)
    .bind(units)
    .fetch_one(&state.write_pool)
    .await
    .map_err(map_write_error)?;
    Ok((StatusCode::CREATED, Json(row_to_response(row)?)))
}

pub async fn update_leave_type(
    Extension(state): Extension<AppState>,
    Extension(actor): Extension<User>,
    Path(code): Path<String>,
    Json(payload): Json<UpdateLeaveTypeRequest>,
) -> Result<Json<LeaveTypeResponse>, AppError> {
    ensure_system_admin(&actor)?;
    payload.validate()?;
    let row = sqlx::query_as::<_, LeaveTypeRow>(
        "UPDATE leave_types
         SET name = $2, is_paid = $3, balance_tracked = $4, allowed_units = $5,
             is_active = $6, updated_at = NOW()
         WHERE code = $1
         RETURNING code, name, is_paid, balance_tracked, allowed_units, is_active",
    )
    .bind(&code)
    .bind(payload.name.trim())
    .bind(payload.is_paid)
    .bind(payload.balance_tracked)
    .bind(units_to_db(&payload.allowed_units))
    .bind(payload.is_active)
    .fetch_optional(&state.write_pool)
    .await
    .map_err(map_write_error)?
    .ok_or_else(|| AppError::NotFound("Leave type not found".into()))?;
    Ok(Json(row_to_response(row)?))
}

pub async fn delete_leave_type(
    Extension(state): Extension<AppState>,
    Extension(actor): Extension<User>,
    Path(code): Path<String>,
) -> Result<StatusCode, AppError> {
    ensure_system_admin(&actor)?;
    if matches!(code.as_str(), "annual" | "sick" | "personal" | "other") {
        return Err(AppError::Conflict(
            "Legacy leave types cannot be deleted; deactivate them instead".into(),
        ));
    }
    let result = sqlx::query("DELETE FROM leave_types WHERE code = $1")
        .bind(code)
        .execute(&state.write_pool)
        .await
        .map_err(map_write_error)?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Leave type not found".into()));
    }
    Ok(StatusCode::NO_CONTENT)
}

fn ensure_system_admin(actor: &User) -> Result<(), AppError> {
    if actor.is_system_admin() {
        Ok(())
    } else {
        Err(AppError::Forbidden("Forbidden".into()))
    }
}

fn validate_code(code: &str) -> Result<(), AppError> {
    let mut chars = code.chars();
    let valid = chars.next().is_some_and(|first| first.is_ascii_lowercase())
        && chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_');
    if valid {
        Ok(())
    } else {
        Err(AppError::BadRequest(
            "code must start with a lowercase letter and contain only lowercase letters, digits, or underscore"
                .into(),
        ))
    }
}

fn units_to_db(units: &[LeaveUnit]) -> Vec<&'static str> {
    units
        .iter()
        .map(|unit| match unit {
            LeaveUnit::Day => "day",
            LeaveUnit::HalfAm => "half_am",
            LeaveUnit::HalfPm => "half_pm",
            LeaveUnit::Hour => "hour",
        })
        .collect()
}

fn row_to_response(row: LeaveTypeRow) -> Result<LeaveTypeResponse, AppError> {
    let allowed_units = row
        .allowed_units
        .into_iter()
        .map(|unit| match unit.as_str() {
            "day" => Ok(LeaveUnit::Day),
            "half_am" => Ok(LeaveUnit::HalfAm),
            "half_pm" => Ok(LeaveUnit::HalfPm),
            "hour" => Ok(LeaveUnit::Hour),
            _ => Err(AppError::InternalServerError(anyhow::anyhow!(
                "unknown leave unit in database"
            ))),
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(LeaveTypeResponse {
        code: row.code,
        name: row.name,
        is_paid: row.is_paid,
        balance_tracked: row.balance_tracked,
        allowed_units,
        is_active: row.is_active,
    })
}

fn map_write_error(error: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(database) = &error {
        match database.code().as_deref() {
            Some("23505") => return AppError::Conflict("Leave type code already exists".into()),
            Some("23503") => return AppError::Conflict("Leave type is currently in use".into()),
            _ => {}
        }
    }
    AppError::from(error)
}
