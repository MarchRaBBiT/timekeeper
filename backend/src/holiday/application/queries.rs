use chrono::{Datelike, NaiveDate};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    error::AppError,
    models::holiday::{CreateHolidayPayload, HolidayResponse},
    repositories::holiday::{HolidayRepository, HolidayRepositoryTrait},
    services::holiday::HolidayServiceTrait,
    types::UserId,
};

const GOOGLE_JP_HOLIDAY_ICS: &str =
    "https://calendar.google.com/calendar/ical/japanese__ja%40holiday.calendar.google.com/public/basic.ics";
const HOLIDAY_DESCRIPTION_PREFIX: &str = "\u{795d}\u{65e5}";

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

pub async fn list_public_holidays(
    read_pool: &sqlx::PgPool,
) -> Result<Vec<HolidayResponse>, AppError> {
    let repo = HolidayRepository::new();
    let holidays = repo.find_all(read_pool).await?;
    Ok(holidays.into_iter().map(HolidayResponse::from).collect())
}

pub async fn fetch_google_holidays(
    year: Option<i32>,
) -> Result<Vec<CreateHolidayPayload>, AppError> {
    let client = Client::builder()
        .user_agent("timekeeper-backend/1.0")
        .build()
        .map_err(|error| {
            AppError::InternalServerError(anyhow::anyhow!(
                "Failed to initialize HTTP client: {}",
                error
            ))
        })?;

    let response = client
        .get(GOOGLE_JP_HOLIDAY_ICS)
        .send()
        .await
        .map_err(|error| {
            AppError::InternalServerError(anyhow::anyhow!(
                "Failed to fetch Google Calendar: {}",
                error
            ))
        })?;

    let response_text = response.text().await.map_err(|error| {
        AppError::InternalServerError(anyhow::anyhow!("Failed to read Google Calendar: {}", error))
    })?;

    Ok(parse_google_calendar_ics(&response_text, year))
}

pub async fn check_holiday(
    holiday_service: Arc<dyn HolidayServiceTrait>,
    user_id: UserId,
    date: NaiveDate,
) -> Result<HolidayCheckResponse, AppError> {
    let decision = holiday_service
        .is_holiday(date, Some(&user_id.to_string()))
        .await
        .map_err(|error| AppError::InternalServerError(error.into()))?;

    Ok(HolidayCheckResponse {
        is_holiday: decision.is_holiday,
        reason: if decision.is_holiday {
            Some(decision.reason.label().to_string())
        } else {
            None
        },
    })
}

pub async fn list_month_holidays(
    holiday_service: Arc<dyn HolidayServiceTrait>,
    user_id: UserId,
    year: i32,
    month: u32,
) -> Result<Vec<HolidayMonthEntry>, AppError> {
    if !(1..=12).contains(&month) {
        return Err(AppError::BadRequest(
            "Month must be between 1 and 12".into(),
        ));
    }

    let entries = holiday_service
        .list_month(year, month, Some(&user_id.to_string()))
        .await
        .map_err(|error| AppError::InternalServerError(error.into()))?;

    Ok(entries
        .into_iter()
        .map(|entry| HolidayMonthEntry {
            date: entry.date,
            reason: entry.reason.label().to_string(),
        })
        .collect())
}

pub fn parse_google_calendar_ics(
    content: &str,
    year_filter: Option<i32>,
) -> Vec<CreateHolidayPayload> {
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
        } else if let Some(stripped) = line.strip_prefix("SUMMARY:") {
            summary = Some(decode_ics_text(stripped));
        } else if let Some(stripped) = line.strip_prefix("DESCRIPTION:") {
            description = Some(decode_ics_text(stripped));
        } else if line.starts_with("END:VEVENT") {
            if let (Some(date), Some(name)) = (current_date, summary.clone()) {
                if year_filter.map(|year| date.year() == year).unwrap_or(true) {
                    let normalized_description = description.clone().and_then(|value| {
                        let trimmed = value.trim();
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

    events.sort_by_key(|holiday| (holiday.holiday_date, holiday.name.clone()));
    events
}

pub fn decode_ics_text(raw: &str) -> String {
    raw.replace("\\n", "\n")
        .replace("\\,", ",")
        .replace("\\;", ";")
        .replace("\\\\", "\\")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_ics_text_decodes_newlines() {
        assert_eq!(decode_ics_text("Line1\\nLine2"), "Line1\nLine2");
    }

    #[test]
    fn decode_ics_text_decodes_commas() {
        assert_eq!(
            decode_ics_text("Value\\,With\\,Commas"),
            "Value,With,Commas"
        );
    }

    #[test]
    fn decode_ics_text_decodes_semicolons() {
        assert_eq!(
            decode_ics_text("Value\\;With\\;Semicolons"),
            "Value;With;Semicolons"
        );
    }

    #[test]
    fn decode_ics_text_decodes_backslashes() {
        assert_eq!(
            decode_ics_text("Value\\\\With\\\\Backslash"),
            "Value\\With\\Backslash"
        );
    }

    #[test]
    fn parse_google_calendar_ics_extracts_holiday_events() {
        let ics_content = r#"BEGIN:VCALENDAR
BEGIN:VEVENT
DTSTART:20240101
SUMMARY:元日
DESCRIPTION:祝日
END:VEVENT
BEGIN:VEVENT
DTSTART:20240108
SUMMARY:成人の日
DESCRIPTION:祝日
END:VEVENT
END:VCALENDAR"#;

        let result = parse_google_calendar_ics(ics_content, None);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "元日");
        assert_eq!(
            result[0].holiday_date,
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
        );
    }

    #[test]
    fn parse_google_calendar_ics_filters_by_year() {
        let ics_content = r#"BEGIN:VCALENDAR
BEGIN:VEVENT
DTSTART:20230101
SUMMARY:元日 2023
DESCRIPTION:祝日
END:VEVENT
BEGIN:VEVENT
DTSTART:20240101
SUMMARY:元日 2024
DESCRIPTION:祝日
END:VEVENT
END:VCALENDAR"#;

        let result = parse_google_calendar_ics(ics_content, Some(2024));
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "元日 2024");
    }
}
