use axum::{extract::State, Json};
use serde::Serialize;

use crate::{config::Config, db::connection::DbPool};

#[derive(Serialize)]
pub struct TimeZoneResponse {
    pub time_zone: String,
}

pub async fn get_time_zone(State((_, config)): State<(DbPool, Config)>) -> Json<TimeZoneResponse> {
    Json(TimeZoneResponse {
        time_zone: config.time_zone.to_string(),
    })
}
