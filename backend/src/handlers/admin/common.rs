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
