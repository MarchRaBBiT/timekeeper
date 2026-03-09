use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::{
    admin::application::attendance_correction_requests as application,
    error::AppError,
    models::attendance_correction_request::{AttendanceCorrectionResponse, DecisionPayload},
    models::user::User,
    state::AppState,
};

#[derive(Debug, Clone, Deserialize)]
pub struct AdminAttendanceCorrectionListQuery {
    pub status: Option<String>,
    pub user_id: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

pub async fn list_attendance_correction_requests(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(query): Query<AdminAttendanceCorrectionListQuery>,
) -> Result<Json<Vec<AttendanceCorrectionResponse>>, AppError> {
    Ok(Json(
        application::list_attendance_correction_requests(
            state.read_pool(),
            &user,
            application::AdminAttendanceCorrectionListQuery {
                status: query.status,
                user_id: query.user_id,
                page: query.page,
                per_page: query.per_page,
            },
        )
        .await?,
    ))
}

pub async fn get_attendance_correction_request_detail(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> Result<Json<AttendanceCorrectionResponse>, AppError> {
    Ok(Json(
        application::get_attendance_correction_request_detail(state.read_pool(), &user, &id)
            .await?,
    ))
}

pub async fn approve_attendance_correction_request(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
    Json(payload): Json<DecisionPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(
        application::approve_attendance_correction_request(&state.write_pool, &user, &id, payload)
            .await?,
    ))
}

pub async fn reject_attendance_correction_request(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
    Json(payload): Json<DecisionPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(
        application::reject_attendance_correction_request(&state.write_pool, &user, &id, payload)
            .await?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attendance_correction_query_holds_optional_filters() {
        let query = AdminAttendanceCorrectionListQuery {
            status: Some("pending".to_string()),
            user_id: Some("user-1".to_string()),
            page: Some(2),
            per_page: Some(50),
        };

        assert_eq!(query.status.as_deref(), Some("pending"));
        assert_eq!(query.user_id.as_deref(), Some("user-1"));
        assert_eq!(query.page, Some(2));
        assert_eq!(query.per_page, Some(50));
    }
}
