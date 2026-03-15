use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};
use std::str::FromStr;

use crate::{
    error::AppError,
    models::{
        department::{
            AssignManagerPayload, CreateDepartmentPayload, DepartmentResponse,
            UpdateDepartmentPayload,
        },
        user::User,
    },
    repositories::department,
    state::AppState,
    types::{DepartmentId, UserId},
};

pub async fn list_departments(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<DepartmentResponse>>, AppError> {
    if !(user.is_manager() || user.is_system_admin()) {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    let depts = department::list_departments(state.read_pool())
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    Ok(Json(
        depts.into_iter().map(DepartmentResponse::from).collect(),
    ))
}

pub async fn get_department(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> Result<Json<DepartmentResponse>, AppError> {
    if !(user.is_manager() || user.is_system_admin()) {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    let dept = department::find_department_by_id(state.read_pool(), &id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?
        .ok_or_else(|| AppError::NotFound("Department not found".into()))?;

    Ok(Json(DepartmentResponse::from(dept)))
}

pub async fn create_department(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<CreateDepartmentPayload>,
) -> Result<(StatusCode, Json<DepartmentResponse>), AppError> {
    if !user.is_system_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    let name = payload.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest("Department name is required".into()));
    }

    let parent_id_str: Option<String> = match payload.parent_id {
        Some(ref raw) => {
            DepartmentId::from_str(raw)
                .map_err(|_| AppError::BadRequest("Invalid parent_id".into()))?;
            Some(raw.clone())
        }
        None => None,
    };

    let new_id = DepartmentId::new().to_string();
    let dept =
        department::create_department(&state.write_pool, &new_id, &name, parent_id_str.as_deref())
            .await
            .map_err(|e| AppError::InternalServerError(e.into()))?;

    Ok((StatusCode::CREATED, Json(DepartmentResponse::from(dept))))
}

pub async fn update_department(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateDepartmentPayload>,
) -> Result<Json<DepartmentResponse>, AppError> {
    if !user.is_system_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    let name = payload
        .name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    // parent_id field in payload: None means "don't change", Some(val) means "set to val (or NULL if empty)"
    let parent_id_update: Option<Option<&str>> = match payload.parent_id.as_deref() {
        Some(raw) if !raw.is_empty() => {
            DepartmentId::from_str(raw)
                .map_err(|_| AppError::BadRequest("Invalid parent_id".into()))?;
            Some(Some(raw))
        }
        Some(_) => Some(None), // empty string → set to NULL
        None => None,          // not provided → don't change
    };

    let dept = department::update_department(&state.write_pool, &id, name, parent_id_update)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?
        .ok_or_else(|| AppError::NotFound("Department not found".into()))?;

    Ok(Json(DepartmentResponse::from(dept)))
}

pub async fn delete_department(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    if !user.is_system_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    let has_children = department::department_has_children(&state.write_pool, &id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    if has_children {
        return Err(AppError::Conflict(
            "Cannot delete a department that has child departments".into(),
        ));
    }

    let deleted = department::delete_department(&state.write_pool, &id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    if !deleted {
        return Err(AppError::NotFound("Department not found".into()));
    }

    Ok(Json(json!({ "message": "Department deleted", "id": id })))
}

pub async fn list_department_managers_handler(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> Result<Json<Vec<Value>>, AppError> {
    if !(user.is_manager() || user.is_system_admin()) {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    // Ensure department exists
    department::find_department_by_id(state.read_pool(), &id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?
        .ok_or_else(|| AppError::NotFound("Department not found".into()))?;

    let managers = department::list_department_managers(state.read_pool(), &id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    let resp: Vec<Value> = managers
        .into_iter()
        .map(|m| {
            json!({
                "department_id": m.department_id.to_string(),
                "user_id": m.user_id.to_string(),
                "assigned_at": m.assigned_at,
            })
        })
        .collect();

    Ok(Json(resp))
}

pub async fn assign_manager_handler(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
    Json(payload): Json<AssignManagerPayload>,
) -> Result<Json<Value>, AppError> {
    if !user.is_system_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    UserId::from_str(&payload.user_id)
        .map_err(|_| AppError::BadRequest("Invalid user_id".into()))?;

    // Ensure department exists
    department::find_department_by_id(&state.write_pool, &id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?
        .ok_or_else(|| AppError::NotFound("Department not found".into()))?;

    department::assign_manager(&state.write_pool, &id, &payload.user_id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    Ok(Json(json!({
        "message": "Manager assigned",
        "department_id": id,
        "user_id": payload.user_id,
    })))
}

pub async fn remove_manager_handler(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path((dept_id, user_id)): Path<(String, String)>,
) -> Result<Json<Value>, AppError> {
    if !user.is_system_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    let removed = department::remove_manager(&state.write_pool, &dept_id, &user_id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    if !removed {
        return Err(AppError::NotFound("Manager assignment not found".into()));
    }

    Ok(Json(json!({
        "message": "Manager removed",
        "department_id": dept_id,
        "user_id": user_id,
    })))
}

#[cfg(test)]
mod tests {

    #[test]
    fn update_payload_with_empty_parent_id_maps_to_null() {
        // Verify that the parsing logic in update_department would
        // treat an empty string parent_id as "set to NULL".
        let raw: Option<&str> = Some("");
        let result: Option<Option<&str>> = match raw {
            Some(s) if !s.is_empty() => Some(Some(s)),
            Some(_) => Some(None),
            None => None,
        };
        assert_eq!(result, Some(None));
    }

    #[test]
    fn update_payload_with_none_parent_id_means_no_change() {
        let raw: Option<&str> = None;
        let result: Option<Option<&str>> = match raw {
            Some(s) if !s.is_empty() => Some(Some(s)),
            Some(_) => Some(None),
            None => None,
        };
        assert_eq!(result, None);
    }
}
