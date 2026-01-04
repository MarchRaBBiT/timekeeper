use axum::{
    http::StatusCode,
    Json,
};
use chrono::NaiveDate;
use serde_json::{json, Value};
use sqlx::{QueryBuilder, Postgres};
use utoipa::ToSchema;

pub const DEFAULT_PAGE: i64 = 1;
pub const DEFAULT_PER_PAGE: i64 = 25;
pub const MAX_PER_PAGE: i64 = 100;
pub const MAX_PAGE: i64 = 1_000;

/// Creates a standard bad request response
pub fn bad_request(message: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": message })))
}

/// Parses a date value from various formats
pub fn parse_date_value(value: &str) -> Option<NaiveDate> {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(value) {
        return Some(dt.date_naive());
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S") {
        return Some(dt.date());
    }
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

/// Parses an optional date value
pub fn parse_optional_date(raw: Option<&str>) -> Result<Option<NaiveDate>, &'static str> {
    match raw {
        Some(value) => parse_date_value(value)
            .ok_or("`from`/`to` must be a valid date (YYYY-MM-DD or RFC3339)")
            .map(Some),
        None => Ok(None),
    }
}

/// Adds a WHERE or AND clause to a query builder
pub fn push_clause(builder: &mut QueryBuilder<'_, Postgres>, has_clause: &mut bool) {
    if *has_clause {
        builder.push(" AND ");
    } else {
        builder.push(" WHERE ");
        *has_clause = true;
    }
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct ExportQuery {
    pub username: Option<String>,
    pub from: Option<String>, // YYYY-MM-DD
    pub to: Option<String>,   // YYYY-MM-DD
}