use chrono::{DateTime, NaiveDate, NaiveDateTime};
use sqlx::{Postgres, QueryBuilder};

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

pub fn push_clause(builder: &mut QueryBuilder<'_, Postgres>, has_clause: &mut bool) {
    if *has_clause {
        builder.push(" AND ");
    } else {
        builder.push(" WHERE ");
        *has_clause = true;
    }
}
