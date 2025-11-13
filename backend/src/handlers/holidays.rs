use axum::{
    extract::{Extension, State},
    http::StatusCode,
    Json,
};
use serde_json::json;
use sqlx::PgPool;

use crate::{
    config::Config,
    models::{
        holiday::{Holiday, HolidayResponse},
        user::User,
    },
};

pub async fn list_public_holidays(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(_user): Extension<User>,
) -> Result<Json<Vec<HolidayResponse>>, (StatusCode, Json<serde_json::Value>)> {
    let holidays = sqlx::query_as::<_, Holiday>(
        "SELECT id, holiday_date, name, description, created_at, updated_at \
         FROM holidays ORDER BY holiday_date",
    )
    .fetch_all(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error":"Database error"})),
        )
    })?;

    Ok(Json(
        holidays
            .into_iter()
            .map(HolidayResponse::from)
            .collect::<Vec<_>>(),
    ))
}
