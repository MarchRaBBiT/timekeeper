use std::{
    collections::{BTreeSet, HashMap},
    future::Future,
    pin::Pin,
    sync::Arc,
};

use chrono::{Datelike, Duration, NaiveDate};
use sqlx::{PgPool, Row};

type SourceLoader = Arc<
    dyn Fn(
            NaiveDate,
            NaiveDate,
            Option<&str>,
        ) -> Pin<Box<dyn Future<Output = sqlx::Result<HolidaySources>> + Send + 'static>>
        + Send
        + Sync,
>;

#[derive(Clone)]
pub struct HolidayService {
    load_sources: SourceLoader,
}

impl HolidayService {
    pub fn new(pool: PgPool) -> Self {
        let load_sources =
            move |window_start: NaiveDate, window_end: NaiveDate, user_id: Option<&str>| {
                let pool = pool.clone();
                let user_id = user_id.map(str::to_owned);
                Box::pin(async move {
                    load_sources_from_db(&pool, window_start, window_end, user_id.as_deref()).await
                })
                    as Pin<Box<dyn Future<Output = sqlx::Result<HolidaySources>> + Send + 'static>>
            };

        Self {
            load_sources: Arc::new(load_sources),
        }
    }

    fn with_loader<F, Fut>(loader: F) -> Self
    where
        F: Fn(NaiveDate, NaiveDate, Option<&str>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = sqlx::Result<HolidaySources>> + Send + 'static,
    {
        Self {
            load_sources: Arc::new(move |window_start, window_end, user_id| {
                let user_id = user_id.map(str::to_owned);
                Box::pin(loader(window_start, window_end, user_id.as_deref()))
                    as Pin<Box<dyn Future<Output = sqlx::Result<HolidaySources>> + Send + 'static>>
            }),
        }
    }

    pub async fn is_holiday(
        &self,
        date: NaiveDate,
        user_id: Option<&str>,
    ) -> sqlx::Result<HolidayDecision> {
        let end = date
            .succ_opt()
            .ok_or_else(|| sqlx::Error::Protocol("date overflow".into()))?;
        let sources = (self.load_sources)(date, end, user_id).await?;
        Ok(sources.decision_for(date))
    }

    pub async fn list_month(
        &self,
        year: i32,
        month: u32,
        user_id: Option<&str>,
    ) -> sqlx::Result<Vec<HolidayCalendarEntry>> {
        let (window_start, window_end) = month_bounds(year, month)?;
        let sources = (self.load_sources)(window_start, window_end, user_id).await?;

        let mut cursor = window_start;
        let mut entries = Vec::new();
        while cursor < window_end {
            if let Some(entry) = sources.entry_for(cursor) {
                entries.push(entry);
            }
            cursor = cursor
                .succ_opt()
                .ok_or_else(|| sqlx::Error::Protocol("date overflow".into()))?;
        }

        Ok(entries)
    }
}

async fn load_sources_from_db(
    pool: &PgPool,
    window_start: NaiveDate,
    window_end: NaiveDate,
    user_id: Option<&str>,
) -> sqlx::Result<HolidaySources> {
    ensure_valid_window(window_start, window_end)?;

    let mut sources = HolidaySources::default();
    let last_inclusive = window_end
        .pred_opt()
        .ok_or_else(|| sqlx::Error::Protocol("invalid calendar window".into()))?;

    let public_rows = sqlx::query(
        r#"
        SELECT holiday_date
        FROM holidays
        WHERE holiday_date >= $1
          AND holiday_date <= $2
        ORDER BY holiday_date
        "#,
    )
    .bind(window_start)
    .bind(last_inclusive)
    .fetch_all(pool)
    .await?;

    for row in public_rows {
        let date: NaiveDate = row.try_get("holiday_date")?;
        sources.public_holidays.insert(date);
    }

    let weekly_rows = sqlx::query(
        r#"
        SELECT weekday, enforced_from, enforced_to
        FROM weekly_holidays
        WHERE enforced_from <= $1
          AND (enforced_to IS NULL OR enforced_to >= $2)
        "#,
    )
    .bind(last_inclusive)
    .bind(window_start)
    .fetch_all(pool)
    .await?;

    for row in weekly_rows {
        let weekday: i16 = row.try_get("weekday")?;
        let enforced_from: NaiveDate = row.try_get("enforced_from")?;
        let enforced_to: Option<NaiveDate> = row.try_get("enforced_to")?;
        let dates = expand_weekly_dates(
            weekday,
            enforced_from,
            enforced_to,
            window_start,
            window_end,
        );
        sources.weekly_holidays.extend(dates);
    }

    if let Some(user_id) = user_id {
        let exception_rows = sqlx::query(
            r#"
            SELECT exception_date, override
            FROM holiday_exceptions
            WHERE user_id = $1
              AND exception_date >= $2
              AND exception_date <= $3
            "#,
        )
        .bind(user_id)
        .bind(window_start)
        .bind(last_inclusive)
        .fetch_all(pool)
        .await?;

        for row in exception_rows {
            let exception_date: NaiveDate = row.try_get("exception_date")?;
            let override_value: bool = row.try_get("override")?;
            sources
                .exception_overrides
                .insert(exception_date, override_value);
        }
    }

    Ok(sources)
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

#[derive(Default, Clone)]
struct HolidaySources {
    public_holidays: BTreeSet<NaiveDate>,
    weekly_holidays: BTreeSet<NaiveDate>,
    exception_overrides: HashMap<NaiveDate, bool>,
}

#[allow(dead_code)]
pub struct HolidayServiceStub {
    sources: Arc<HolidaySources>,
}

#[allow(dead_code)]
impl HolidayServiceStub {
    pub fn new(
        public_holidays: impl IntoIterator<Item = NaiveDate>,
        weekly_holidays: impl IntoIterator<Item = NaiveDate>,
        exception_overrides: impl IntoIterator<Item = (NaiveDate, bool)>,
    ) -> Self {
        let sources = HolidaySources {
            public_holidays: public_holidays.into_iter().collect(),
            weekly_holidays: weekly_holidays.into_iter().collect(),
            exception_overrides: exception_overrides.into_iter().collect(),
        };

        Self {
            sources: Arc::new(sources),
        }
    }

    pub fn service(&self) -> HolidayService {
        let sources = Arc::clone(&self.sources);

        HolidayService::with_loader(move |window_start, window_end, _| {
            let sources = Arc::clone(&sources);

            async move {
                ensure_valid_window(window_start, window_end)?;
                Ok((*sources).clone())
            }
        })
    }
}

impl HolidaySources {
    fn decision_for(&self, date: NaiveDate) -> HolidayDecision {
        if let Some(&flag) = self.exception_overrides.get(&date) {
            return HolidayDecision {
                is_holiday: flag,
                reason: HolidayReason::ExceptionOverride,
            };
        }

        if self.public_holidays.contains(&date) {
            return HolidayDecision {
                is_holiday: true,
                reason: HolidayReason::PublicHoliday,
            };
        }

        if self.weekly_holidays.contains(&date) {
            return HolidayDecision {
                is_holiday: true,
                reason: HolidayReason::WeeklyHoliday,
            };
        }

        HolidayDecision {
            is_holiday: false,
            reason: HolidayReason::None,
        }
    }

    fn entry_for(&self, date: NaiveDate) -> Option<HolidayCalendarEntry> {
        let decision = self.decision_for(date);
        decision.is_holiday.then_some(HolidayCalendarEntry {
            date,
            is_holiday: true,
            reason: decision.reason,
        })
    }
}

fn month_bounds(year: i32, month: u32) -> sqlx::Result<(NaiveDate, NaiveDate)> {
    let start = NaiveDate::from_ymd_opt(year, month, 1)
        .ok_or_else(|| sqlx::Error::Protocol(format!("invalid year/month: {}/{}", year, month)))?;

    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };

    let end = NaiveDate::from_ymd_opt(next_year, next_month, 1).ok_or_else(|| {
        sqlx::Error::Protocol(format!("invalid year/month: {}/{}", next_year, next_month))
    })?;

    Ok((start, end))
}

fn ensure_valid_window(window_start: NaiveDate, window_end: NaiveDate) -> sqlx::Result<()> {
    if window_start >= window_end {
        Err(sqlx::Error::Protocol(
            "invalid calendar window: start must be before end".into(),
        ))
    } else {
        Ok(())
    }
}

fn expand_weekly_dates(
    weekday: i16,
    enforced_from: NaiveDate,
    enforced_to: Option<NaiveDate>,
    window_start: NaiveDate,
    window_end: NaiveDate,
) -> Vec<NaiveDate> {
    let mut result = Vec::new();
    if window_start >= window_end {
        return result;
    }

    let target_weekday = (weekday.rem_euclid(7)) as u32;
    let mut effective_start = enforced_from.max(window_start);
    let last_inclusive = match enforced_to {
        Some(limit) => limit.min(
            window_end
                .pred_opt()
                .expect("window_end must be greater than start"),
        ),
        None => window_end
            .pred_opt()
            .expect("window_end must be greater than start"),
    };

    if effective_start > last_inclusive {
        return result;
    }

    effective_start = align_weekday_on_or_after(effective_start, target_weekday);
    if effective_start > last_inclusive {
        return result;
    }

    let mut cursor = effective_start;
    while cursor <= last_inclusive {
        result.push(cursor);
        cursor += Duration::days(7);
    }

    result
}

fn align_weekday_on_or_after(date: NaiveDate, weekday: u32) -> NaiveDate {
    let current = date.weekday().num_days_from_monday();
    let diff = (weekday + 7 - current) % 7;
    date + Duration::days(diff as i64)
}
