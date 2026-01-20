use axum::{extract::State, Json};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct TimeZoneResponse {
    pub time_zone: String,
}

pub async fn get_time_zone(State(state): State<AppState>) -> Json<TimeZoneResponse> {
    Json(TimeZoneResponse {
        time_zone: state.config.time_zone.to_string(),
    })
}
