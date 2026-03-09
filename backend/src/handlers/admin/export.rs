use axum::{
    extract::{Extension, Query, State},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;
use utoipa::{IntoParams, ToSchema};

use crate::{
    admin::application::export as application,
    application::clock::{Clock, SYSTEM_CLOCK},
    error::AppError,
    models::user::User,
    state::AppState,
};

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct ExportQuery {
    pub username: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

pub async fn export_data(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(q): Query<ExportQuery>,
) -> Result<impl IntoResponse, AppError> {
    let filename = format!(
        "attendance_export_{}.csv",
        SYSTEM_CLOCK
            .now_in_timezone(&state.config.time_zone)
            .format("%Y%m%d_%H%M%S")
    );

    let (headers, response) = application::export_attendance_data(
        state.read_pool(),
        &state.config,
        &user,
        application::ExportQuery {
            username: q.username,
            from: q.from,
            to: q.to,
        },
        filename,
    )
    .await?;

    Ok((
        headers,
        Json(json!({
            "csv_data": response.csv_data,
            "filename": response.filename,
        })),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_query_holds_optional_filters() {
        let query = ExportQuery {
            username: Some("alice".to_string()),
            from: Some("2026-03-01".to_string()),
            to: Some("2026-03-31".to_string()),
        };

        assert_eq!(query.username.as_deref(), Some("alice"));
        assert_eq!(query.from.as_deref(), Some("2026-03-01"));
        assert_eq!(query.to.as_deref(), Some("2026-03-31"));
    }
}
