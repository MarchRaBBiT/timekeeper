use chrono::{Datelike, NaiveDate};
use sqlx::PgPool;

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
                let reason = if exception.r#override {
                    HolidayReason::ForcedHolidayOverride
                } else {
                    HolidayReason::WorkingDayOverride
                };

                return Ok(HolidayDecision {
                    is_holiday: exception.r#override,
                    reason,
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

    pub async fn list_month(
        &self,
        year: i32,
        month: u32,
        user_id: Option<&str>,
    ) -> sqlx::Result<Vec<HolidayCalendarEntry>> {
        let start = NaiveDate::from_ymd_opt(year, month, 1).ok_or_else(|| {
            sqlx::Error::Protocol(format!("invalid year/month: {}/{}", year, month))
        })?;

        let mut cursor = start;
        let mut entries = Vec::new();

        loop {
            if cursor.month() != month {
                break;
            }

            let decision = self.is_holiday(cursor, user_id).await?;
            let include = match decision.reason {
                HolidayReason::None => decision.is_holiday,
                _ => true,
            };

            if include {
                entries.push(HolidayCalendarEntry {
                    date: cursor,
                    is_holiday: decision.is_holiday,
                    reason: decision.reason,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HolidayDecision {
    pub is_holiday: bool,
    pub reason: HolidayReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HolidayReason {
    PublicHoliday,
    WeeklyHoliday,
    ForcedHolidayOverride,
    WorkingDayOverride,
    None,
}

impl HolidayReason {
    pub fn label(&self) -> &'static str {
        match self {
            HolidayReason::PublicHoliday => "public holiday",
            HolidayReason::WeeklyHoliday => "weekly holiday",
            HolidayReason::ForcedHolidayOverride => "forced holiday",
            HolidayReason::WorkingDayOverride => "working day",
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
