use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use sqlx::{PgPool, Postgres, QueryBuilder};

use crate::{error::AppError, models::user::User, types::UserId};

pub fn parse_date_value(value: &str) -> Option<NaiveDate> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        return Some(dt.date_naive());
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S") {
        return Some(dt.date());
    }
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

pub fn parse_optional_date(raw: Option<&str>) -> Result<Option<NaiveDate>, &'static str> {
    match raw {
        Some(value) => parse_date_value(value)
            .ok_or("`from`/`to` must be a valid date (YYYY-MM-DD or RFC3339)")
            .map(Some),
        None => Ok(None),
    }
}

pub fn normalize_filter(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn parse_filter_datetime(value: &str, end_of_day: bool) -> Option<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        return Some(dt.with_timezone(&Utc));
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S") {
        return Some(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S") {
        return Some(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
    }
    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        let time = if end_of_day {
            NaiveTime::from_hms_opt(23, 59, 59)
        } else {
            NaiveTime::from_hms_opt(0, 0, 0)
        }?;
        return Some(DateTime::<Utc>::from_naive_utc_and_offset(
            NaiveDateTime::new(date, time),
            Utc,
        ));
    }
    None
}

pub fn parse_from_datetime(raw: Option<&str>) -> Result<Option<DateTime<Utc>>, &'static str> {
    match raw {
        Some(value) => parse_filter_datetime(value, false)
            .ok_or("`from` must be a valid datetime (RFC3339 or YYYY-MM-DD)")
            .map(Some),
        None => Ok(None),
    }
}

pub fn parse_to_datetime(raw: Option<&str>) -> Result<Option<DateTime<Utc>>, &'static str> {
    match raw {
        Some(value) => parse_filter_datetime(value, true)
            .ok_or("`to` must be a valid datetime (RFC3339 or YYYY-MM-DD)")
            .map(Some),
        None => Ok(None),
    }
}

/// Checks whether `actor` is authorized to approve/reject a request submitted by `applicant_id`.
///
/// Authorization rules:
/// 1. `is_system_admin` → always authorized.
/// 2. `is_manager` → authorized only if applicant is in actor's direct or subordinate departments.
/// 3. Otherwise → 403 Forbidden.
pub async fn check_approval_authorization(
    pool: &PgPool,
    actor: &User,
    applicant_id: UserId,
) -> Result<(), AppError> {
    if actor.is_system_admin() {
        return Ok(());
    }
    if actor.is_manager() {
        let can_approve =
            crate::repositories::department::can_manager_approve(pool, actor.id, applicant_id)
                .await
                .map_err(|e| AppError::InternalServerError(e.into()))?;
        if can_approve {
            return Ok(());
        }
        return Err(AppError::Forbidden(
            "Manager does not have permission to approve this request".into(),
        ));
    }
    Err(AppError::Forbidden("Forbidden".into()))
}

pub fn push_clause(builder: &mut QueryBuilder<'_, Postgres>, has_clause: &mut bool) {
    if *has_clause {
        builder.push(" AND ");
    } else {
        builder.push(" WHERE ");
        *has_clause = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_date_value_supports_rfc3339_sql_and_plain_date() {
        let rfc = parse_date_value("2026-02-04T09:10:11+09:00").expect("parse rfc3339");
        assert_eq!(
            rfc,
            NaiveDate::from_ymd_opt(2026, 2, 4).expect("valid date")
        );

        let sql = parse_date_value("2026-02-04 09:10:11").expect("parse sql datetime");
        assert_eq!(
            sql,
            NaiveDate::from_ymd_opt(2026, 2, 4).expect("valid date")
        );

        let plain = parse_date_value("2026-02-04").expect("parse date");
        assert_eq!(
            plain,
            NaiveDate::from_ymd_opt(2026, 2, 4).expect("valid date")
        );
    }

    #[test]
    fn parse_date_value_returns_none_for_invalid_values() {
        assert!(parse_date_value("not-a-date").is_none());
        assert!(parse_date_value("2026-13-01").is_none());
    }

    #[test]
    fn parse_optional_date_handles_none_and_invalid() {
        assert!(parse_optional_date(None)
            .expect("none should be ok")
            .is_none());
        assert!(parse_optional_date(Some("2026-02-04"))
            .expect("valid optional date")
            .is_some());
        assert!(parse_optional_date(Some("invalid")).is_err());
    }

    #[test]
    fn normalize_filter_trims_and_drops_empty_values() {
        assert_eq!(
            normalize_filter(Some("  user-1  ".to_string())),
            Some("user-1".to_string())
        );
        assert_eq!(normalize_filter(Some("   ".to_string())), None);
        assert_eq!(normalize_filter(None), None);
    }

    #[test]
    fn parse_filter_datetime_supports_rfc3339_sql_iso_and_date_only() {
        assert!(parse_filter_datetime("2026-02-04T10:11:12+09:00", false).is_some());
        assert!(parse_filter_datetime("2026-02-04 10:11:12", false).is_some());
        assert!(parse_filter_datetime("2026-02-04T10:11:12", false).is_some());

        let from = parse_filter_datetime("2026-02-04", false).expect("from");
        assert_eq!(from.time(), NaiveTime::from_hms_opt(0, 0, 0).expect("time"));
        let to = parse_filter_datetime("2026-02-04", true).expect("to");
        assert_eq!(
            to.time(),
            NaiveTime::from_hms_opt(23, 59, 59).expect("time")
        );
    }

    #[test]
    fn push_clause_switches_between_where_and_and() {
        let mut builder: QueryBuilder<'_, Postgres> = QueryBuilder::new("SELECT 1");
        let mut has_clause = false;

        push_clause(&mut builder, &mut has_clause);
        builder.push("a = 1");
        assert!(has_clause);

        push_clause(&mut builder, &mut has_clause);
        builder.push("b = 2");

        assert_eq!(builder.sql(), "SELECT 1 WHERE a = 1 AND b = 2");
    }
}
