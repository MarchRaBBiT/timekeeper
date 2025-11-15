use axum::{
    extract::{Extension, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{Datelike, NaiveDate};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;

use crate::{
    config::Config,
    models::{
        holiday::{CreateHolidayPayload, Holiday, HolidayResponse},
        user::User,
    },
    services::holiday::HolidayService,
};

const GOOGLE_JP_HOLIDAY_ICS: &str =
    "https://calendar.google.com/calendar/ical/japanese__ja%40holiday.calendar.google.com/public/basic.ics";
const HOLIDAY_DESCRIPTION_PREFIX: &str = "\u{795d}\u{65e5}"; // Japanese word for "holiday"

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

#[derive(Debug, Deserialize)]
pub struct GoogleHolidayQuery {
    pub year: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct HolidayCheckQuery {
    pub date: NaiveDate,
}

#[derive(Debug, Deserialize)]
pub struct HolidayMonthQuery {
    pub year: i32,
    pub month: u32,
}

#[derive(Debug, Serialize)]
pub struct HolidayCheckResponse {
    pub is_holiday: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HolidayMonthEntry {
    pub date: NaiveDate,
    pub reason: String,
}

pub async fn fetch_google_holidays(
    State((_pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Query(params): Query<GoogleHolidayQuery>,
) -> Result<Json<Vec<CreateHolidayPayload>>, (StatusCode, Json<serde_json::Value>)> {
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }

    let client = Client::builder()
        .user_agent("timekeeper-backend/1.0")
        .build()
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error":"Failed to initialize HTTP client"})),
            )
        })?;

    let response_text = client
        .get(GOOGLE_JP_HOLIDAY_ICS)
        .send()
        .await
        .map_err(|_| {
            (
                StatusCode::BAD_GATEWAY,
                Json(json!({"error":"Failed to fetch Google Calendar"})),
            )
        })?
        .text()
        .await
        .map_err(|_| {
            (
                StatusCode::BAD_GATEWAY,
                Json(json!({"error":"Failed to read Google Calendar"})),
            )
        })?;

    let parsed = parse_google_calendar_ics(&response_text, params.year);
    Ok(Json(parsed))
}

pub async fn check_holiday(
    Extension(user): Extension<User>,
    Extension(holiday_service): Extension<Arc<HolidayService>>,
    Query(query): Query<HolidayCheckQuery>,
) -> Result<Json<HolidayCheckResponse>, (StatusCode, Json<serde_json::Value>)> {
    let decision = holiday_service
        .is_holiday(query.date, Some(&user.id))
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error":"Failed to evaluate holiday calendar"})),
            )
        })?;

    let reason = if decision.is_holiday {
        Some(decision.reason.label().to_string())
    } else {
        None
    };

    Ok(Json(HolidayCheckResponse {
        is_holiday: decision.is_holiday,
        reason,
    }))
}

pub async fn list_month_holidays(
    Extension(user): Extension<User>,
    Extension(holiday_service): Extension<Arc<HolidayService>>,
    Query(query): Query<HolidayMonthQuery>,
) -> Result<Json<Vec<HolidayMonthEntry>>, (StatusCode, Json<serde_json::Value>)> {
    if !(1..=12).contains(&query.month) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"Month must be between 1 and 12"})),
        ));
    }

    let entries = holiday_service
        .list_month(query.year, query.month, Some(&user.id))
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error":"Failed to load holiday calendar"})),
            )
        })?;

    let response = entries
        .into_iter()
        .map(|entry| HolidayMonthEntry {
            date: entry.date,
            reason: entry.reason.label().to_string(),
        })
        .collect();

    Ok(Json(response))
}

fn parse_google_calendar_ics(content: &str, year_filter: Option<i32>) -> Vec<CreateHolidayPayload> {
    let mut unfolded: Vec<String> = Vec::new();
    for line in content.lines() {
        if let Some(last) = unfolded.last_mut() {
            if line.starts_with(' ') || line.starts_with('\t') {
                last.push_str(line.trim_start());
                continue;
            }
        }
        unfolded.push(line.to_string());
    }

    let mut events = Vec::new();
    let mut current_date: Option<NaiveDate> = None;
    let mut summary: Option<String> = None;
    let mut description: Option<String> = None;

    for line in unfolded {
        if line.starts_with("BEGIN:VEVENT") {
            current_date = None;
            summary = None;
            description = None;
        } else if line.starts_with("DTSTART") {
            if let Some(pos) = line.find(':') {
                let value = &line[pos + 1..];
                if let Ok(date) = NaiveDate::parse_from_str(value, "%Y%m%d") {
                    current_date = Some(date);
                }
            }
        } else if line.starts_with("SUMMARY:") {
            summary = Some(decode_ics_text(&line["SUMMARY:".len()..]));
        } else if line.starts_with("DESCRIPTION:") {
            description = Some(decode_ics_text(&line["DESCRIPTION:".len()..]));
        } else if line.starts_with("END:VEVENT") {
            if let (Some(date), Some(name)) = (current_date, summary.clone()) {
                if year_filter.map(|y| date.year() == y).unwrap_or(true) {
                    let normalized_description = description.clone().and_then(|d| {
                        let trimmed = d.trim();
                        if trimmed.is_empty() {
                            None
                        } else {
                            Some(trimmed.to_string())
                        }
                    });

                    let is_public_holiday = normalized_description
                        .as_deref()
                        .map(|desc| desc.starts_with(HOLIDAY_DESCRIPTION_PREFIX))
                        .unwrap_or(false);

                    if is_public_holiday {
                        events.push(CreateHolidayPayload {
                            holiday_date: date,
                            name: name.trim().to_string(),
                            description: normalized_description,
                        });
                    } else {
                        tracing::debug!(
                            "Skipping non-holiday calendar event: {} ({:?})",
                            name,
                            normalized_description
                        );
                    }
                }
            }
            current_date = None;
            summary = None;
            description = None;
        }
    }

    events.sort_by_key(|h| (h.holiday_date, h.name.clone()));
    events
}

fn decode_ics_text(raw: &str) -> String {
    raw.replace("\\n", "\n")
        .replace("\\,", ",")
        .replace("\\;", ";")
        .replace("\\\\", "\\")
}
