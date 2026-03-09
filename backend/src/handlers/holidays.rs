use axum::{
    extract::{Extension, Query, State},
    Json,
};
use chrono::NaiveDate;
use std::sync::Arc;

use crate::{
    error::AppError,
    holiday::application::queries::{
        check_holiday as check_holiday_use_case, decode_ics_text,
        fetch_google_holidays as fetch_google_holidays_use_case,
        list_month_holidays as list_month_holidays_use_case,
        list_public_holidays as list_public_holidays_use_case, parse_google_calendar_ics,
        GoogleHolidayQuery, HolidayCheckQuery, HolidayCheckResponse, HolidayMonthEntry,
        HolidayMonthQuery,
    },
    models::{
        holiday::{CreateHolidayPayload, HolidayResponse},
        user::User,
    },
    services::holiday::HolidayServiceTrait,
    state::AppState,
};

pub async fn list_public_holidays(
    State(state): State<AppState>,
    Extension(_user): Extension<User>,
) -> Result<Json<Vec<HolidayResponse>>, AppError> {
    Ok(Json(
        list_public_holidays_use_case(state.read_pool()).await?,
    ))
}

pub async fn fetch_google_holidays(
    State(_state): State<AppState>,
    Extension(user): Extension<User>,
    Query(params): Query<GoogleHolidayQuery>,
) -> Result<Json<Vec<CreateHolidayPayload>>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }
    Ok(Json(fetch_google_holidays_use_case(params.year).await?))
}

pub async fn check_holiday(
    Extension(user): Extension<User>,
    Extension(holiday_service): Extension<Arc<dyn HolidayServiceTrait>>,
    Query(query): Query<HolidayCheckQuery>,
) -> Result<Json<HolidayCheckResponse>, AppError> {
    Ok(Json(
        check_holiday_use_case(holiday_service, user.id, query.date).await?,
    ))
}

pub async fn list_month_holidays(
    Extension(user): Extension<User>,
    Extension(holiday_service): Extension<Arc<dyn HolidayServiceTrait>>,
    Query(query): Query<HolidayMonthQuery>,
) -> Result<Json<Vec<HolidayMonthEntry>>, AppError> {
    Ok(Json(
        list_month_holidays_use_case(holiday_service, user.id, query.year, query.month).await?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_ics_text_decodes_newlines() {
        let input = "Line1\\nLine2";
        let result = decode_ics_text(input);
        assert_eq!(result, "Line1\nLine2");
    }

    #[test]
    fn decode_ics_text_decodes_commas() {
        let input = "Value\\,With\\,Commas";
        let result = decode_ics_text(input);
        assert_eq!(result, "Value,With,Commas");
    }

    #[test]
    fn decode_ics_text_decodes_semicolons() {
        let input = "Value\\;With\\;Semicolons";
        let result = decode_ics_text(input);
        assert_eq!(result, "Value;With;Semicolons");
    }

    #[test]
    fn decode_ics_text_decodes_backslashes() {
        let input = "Value\\\\With\\\\Backslash";
        let result = decode_ics_text(input);
        assert_eq!(result, "Value\\With\\Backslash");
    }

    #[test]
    fn decode_ics_text_decodes_mixed() {
        let input = "Line1\\nLine2\\,with\\;special\\\\chars";
        let result = decode_ics_text(input);
        assert_eq!(result, "Line1\nLine2,with;special\\chars");
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
        assert_eq!(result[1].name, "成人の日");
        assert_eq!(
            result[1].holiday_date,
            NaiveDate::from_ymd_opt(2024, 1, 8).unwrap()
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

    #[test]
    fn parse_google_calendar_ics_skips_non_holiday_events() {
        let ics_content = r#"BEGIN:VCALENDAR
BEGIN:VEVENT
DTSTART:20240101
SUMMARY:Regular Meeting
DESCRIPTION:Some other event
END:VEVENT
BEGIN:VEVENT
DTSTART:20240108
SUMMARY:成人の日
DESCRIPTION:祝日
END:VEVENT
END:VCALENDAR"#;

        let result = parse_google_calendar_ics(ics_content, None);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "成人の日");
    }

    #[test]
    fn parse_google_calendar_ics_handles_continuation_lines() {
        let ics_content = r#"BEGIN:VCALENDAR
BEGIN:VEVENT
DTSTART:20240101
SUMMARY:Very Long Holiday
 Name That Continues
DESCRIPTION:祝日
END:VEVENT
END:VCALENDAR"#;

        let result = parse_google_calendar_ics(ics_content, None);

        assert_eq!(result.len(), 1);
        // ICS continuation lines concatenate without adding a space
        assert_eq!(result[0].name, "Very Long HolidayName That Continues");
    }

    #[test]
    fn parse_google_calendar_ics_sorts_by_date() {
        let ics_content = r#"BEGIN:VCALENDAR
BEGIN:VEVENT
DTSTART:20241225
SUMMARY:Christmas
DESCRIPTION:祝日
END:VEVENT
BEGIN:VEVENT
DTSTART:20240101
SUMMARY:元日
DESCRIPTION:祝日
END:VEVENT
BEGIN:VEVENT
DTSTART:20240704
SUMMARY:Independence Day
DESCRIPTION:祝日
END:VEVENT
END:VCALENDAR"#;

        let result = parse_google_calendar_ics(ics_content, None);

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].name, "元日");
        assert_eq!(result[1].name, "Independence Day");
        assert_eq!(result[2].name, "Christmas");
    }
}
