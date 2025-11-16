use chrono::{Datelike, Months, NaiveDate};
use sqlx::{PgPool, Row};
use std::collections::{HashMap, HashSet};

use crate::models::holiday::Holiday;

#[derive(Clone)]
pub struct HolidayService {
    pool: PgPool,
}

impl HolidayService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn is_holiday(
        &self,
        date: NaiveDate,
        user_id: Option<&str>,
    ) -> sqlx::Result<HolidayDecision> {
        if let Some(user_id) = user_id {
            if let Some(exception) = sqlx::query!(
                r#"
                SELECT override
                FROM holiday_exceptions
                WHERE user_id = $1 AND exception_date = $2
                "#,
                user_id,
                date
            )
            .fetch_optional(&self.pool)
            .await?
            {
                return Ok(HolidayDecision {
                    is_holiday: exception.r#override,
                    reason: HolidayReason::ExceptionOverride,
                });
            }
        }

        let public = sqlx::query_scalar!(
            r#"
            SELECT 1 as value
            FROM holidays
            WHERE holiday_date = $1
            LIMIT 1
            "#,
            date
        )
        .fetch_optional(&self.pool)
        .await?;

        if public.is_some() {
            return Ok(HolidayDecision {
                is_holiday: true,
                reason: HolidayReason::PublicHoliday,
            });
        }

        let weekday = date.weekday().num_days_from_monday() as i16;
        let weekly = sqlx::query_scalar!(
            r#"
            SELECT 1 as value
            FROM weekly_holidays
            WHERE weekday = $1
              AND enforced_from <= $2
              AND (enforced_to IS NULL OR enforced_to >= $2)
            LIMIT 1
            "#,
            weekday,
            date
        )
        .fetch_optional(&self.pool)
        .await?;

        if weekly.is_some() {
            return Ok(HolidayDecision {
                is_holiday: true,
                reason: HolidayReason::WeeklyHoliday,
            });
        }

        Ok(HolidayDecision {
            is_holiday: false,
            reason: HolidayReason::None,
        })
    }

    pub async fn list_public_holidays(&self) -> sqlx::Result<Vec<Holiday>> {
        let rows = sqlx::query_as::<_, Holiday>(
            "SELECT id, holiday_date, name, description, created_at, updated_at \
             FROM holidays ORDER BY holiday_date",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn list_month(
        &self,
        year: i32,
        month: u32,
        user_id: Option<&str>,
    ) -> sqlx::Result<Vec<HolidayCalendarEntry>> {
        let start = NaiveDate::from_ymd_opt(year, month, 1).ok_or_else(|| {
            sqlx::Error::Protocol(format!("invalid year/month: {}/{}", year, month))
        })?;
        let end = start
            .checked_add_months(Months::new(1))
            .and_then(|d| d.pred_opt())
            .ok_or_else(|| sqlx::Error::Protocol("failed to calculate month range".into()))?;

        let public_dates = self.public_holiday_set(start, end).await?;
        let weekly_rules = self.weekly_rules_for_range(start, end).await?;
        let exceptions = if let Some(user) = user_id {
            self.user_exception_map(user, start, end).await?
        } else {
            HashMap::new()
        };

        let mut cursor = start;
        let mut entries = Vec::new();

        while cursor.month() == month {
            if let Some(override_flag) = exceptions.get(&cursor) {
                if *override_flag {
                    entries.push(HolidayCalendarEntry {
                        date: cursor,
                        is_holiday: true,
                        reason: HolidayReason::ExceptionOverride,
                    });
                }
            } else if public_dates.contains(&cursor) {
                entries.push(HolidayCalendarEntry {
                    date: cursor,
                    is_holiday: true,
                    reason: HolidayReason::PublicHoliday,
                });
            } else if weekly_rules.iter().any(|rule| rule.matches(cursor)) {
                entries.push(HolidayCalendarEntry {
                    date: cursor,
                    is_holiday: true,
                    reason: HolidayReason::WeeklyHoliday,
                });
            }

            cursor = match cursor.succ_opt() {
                Some(next) => next,
                None => break,
            };
        }

        Ok(entries)
    }
}

struct WeeklyRule {
    weekday: i16,
    enforced_from: NaiveDate,
    enforced_to: Option<NaiveDate>,
}

impl WeeklyRule {
    fn matches(&self, date: NaiveDate) -> bool {
        if self.weekday != date.weekday().num_days_from_monday() as i16 {
            return false;
        }
        if date < self.enforced_from {
            return false;
        }
        if let Some(end) = self.enforced_to {
            if date > end {
                return false;
            }
        }
        true
    }
}

impl HolidayService {
    async fn public_holiday_set(
        &self,
        start: NaiveDate,
        end: NaiveDate,
    ) -> sqlx::Result<HashSet<NaiveDate>> {
        let rows =
            sqlx::query("SELECT holiday_date FROM holidays WHERE holiday_date BETWEEN $1 AND $2")
                .bind(start)
                .bind(end)
                .fetch_all(&self.pool)
                .await?;
        Ok(rows
            .into_iter()
            .filter_map(|row| row.try_get::<NaiveDate, _>("holiday_date").ok())
            .collect())
    }

    async fn weekly_rules_for_range(
        &self,
        start: NaiveDate,
        end: NaiveDate,
    ) -> sqlx::Result<Vec<WeeklyRule>> {
        let rows = sqlx::query(
            "SELECT weekday, enforced_from, enforced_to FROM weekly_holidays \
             WHERE enforced_from <= $1 AND (enforced_to IS NULL OR enforced_to >= $2)",
        )
        .bind(end)
        .bind(start)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| WeeklyRule {
                weekday: row.try_get("weekday").unwrap_or(0),
                enforced_from: row.try_get("enforced_from").unwrap_or(start),
                enforced_to: row.try_get("enforced_to").ok(),
            })
            .collect())
    }

    async fn user_exception_map(
        &self,
        user_id: &str,
        start: NaiveDate,
        end: NaiveDate,
    ) -> sqlx::Result<HashMap<NaiveDate, bool>> {
        let rows = sqlx::query(
            "SELECT exception_date, override FROM holiday_exceptions WHERE user_id = $1 AND exception_date BETWEEN $2 AND $3"
        )
        .bind(user_id)
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .filter_map(|row| {
                let date = row.try_get("exception_date").ok()?;
                let flag = row.try_get("override").unwrap_or(false);
                Some((date, flag))
            })
            .collect())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HolidayDecision {
    pub is_holiday: bool,
    pub reason: HolidayReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HolidayReason {
    PublicHoliday,
    WeeklyHoliday,
    ExceptionOverride,
    None,
}

impl HolidayReason {
    pub fn label(&self) -> &'static str {
        match self {
            HolidayReason::PublicHoliday => "public holiday",
            HolidayReason::WeeklyHoliday => "weekly holiday",
            HolidayReason::ExceptionOverride => "forced holiday",
            HolidayReason::None => "working day",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HolidayCalendarEntry {
    pub date: NaiveDate,
    pub is_holiday: bool,
    pub reason: HolidayReason,
}
