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
}

#[async_trait::async_trait]
pub trait HolidayServiceTrait: Send + Sync {
    async fn is_holiday(
        &self,
        date: NaiveDate,
        user_id: Option<&str>,
    ) -> sqlx::Result<HolidayDecision>;

    async fn list_month(
        &self,
        year: i32,
        month: u32,
        user_id: Option<&str>,
    ) -> sqlx::Result<Vec<HolidayCalendarEntry>>;
}

#[async_trait::async_trait]
impl HolidayServiceTrait for HolidayService {
    async fn is_holiday(
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

    async fn list_month(
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
    let current = date.weekday().num_days_from_sunday();
    let diff = (weekday + 7 - current) % 7;
    date + Duration::days(diff as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holiday_reason_label_returns_correct_strings() {
        assert_eq!(HolidayReason::PublicHoliday.label(), "public holiday");
        assert_eq!(HolidayReason::WeeklyHoliday.label(), "weekly holiday");
        assert_eq!(HolidayReason::ExceptionOverride.label(), "forced holiday");
        assert_eq!(HolidayReason::None.label(), "working day");
    }

    #[test]
    fn month_bounds_returns_correct_range() {
        let result = month_bounds(2024, 1).unwrap();
        assert_eq!(result.0, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        assert_eq!(result.1, NaiveDate::from_ymd_opt(2024, 2, 1).unwrap());
    }

    #[test]
    fn month_bounds_handles_december() {
        let result = month_bounds(2024, 12).unwrap();
        assert_eq!(result.0, NaiveDate::from_ymd_opt(2024, 12, 1).unwrap());
        assert_eq!(result.1, NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
    }

    #[test]
    fn month_bounds_invalid_month_returns_error() {
        let result = month_bounds(2024, 13);
        assert!(result.is_err());
    }

    #[test]
    fn ensure_valid_window_accepts_valid_range() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 2, 1).unwrap();
        assert!(ensure_valid_window(start, end).is_ok());
    }

    #[test]
    fn ensure_valid_window_rejects_invalid_range() {
        let start = NaiveDate::from_ymd_opt(2024, 2, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        assert!(ensure_valid_window(start, end).is_err());
    }

    #[test]
    fn ensure_valid_window_rejects_equal_dates() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        assert!(ensure_valid_window(date, date).is_err());
    }

    #[test]
    fn align_weekday_on_or_after_same_day() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 8).unwrap();
        let result = align_weekday_on_or_after(date, 1);
        assert_eq!(result, date);
    }

    #[test]
    fn align_weekday_on_or_after_future_day() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 8).unwrap();
        let result = align_weekday_on_or_after(date, 3);
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 10).unwrap());
    }

    #[test]
    fn align_weekday_on_or_after_previous_day() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 8).unwrap();
        let result = align_weekday_on_or_after(date, 0);
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 14).unwrap());
    }

    #[test]
    fn expand_weekly_dates_empty_window() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let result = expand_weekly_dates(0, start, None, start, end);
        assert!(result.is_empty());
    }

    #[test]
    fn expand_weekly_dates_no_matches() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 10).unwrap();
        let enforced_from = NaiveDate::from_ymd_opt(2024, 2, 1).unwrap();
        let result = expand_weekly_dates(0, enforced_from, None, start, end);
        assert!(result.is_empty());
    }

    #[test]
    fn holiday_sources_decision_for_exception_override_true() {
        let mut sources = HolidaySources::default();
        sources
            .exception_overrides
            .insert(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(), true);

        let decision = sources.decision_for(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        assert!(decision.is_holiday);
        assert_eq!(decision.reason, HolidayReason::ExceptionOverride);
    }

    #[test]
    fn holiday_sources_decision_for_exception_override_false() {
        let mut sources = HolidaySources::default();
        sources
            .exception_overrides
            .insert(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(), false);

        let decision = sources.decision_for(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        assert!(!decision.is_holiday);
        assert_eq!(decision.reason, HolidayReason::ExceptionOverride);
    }

    #[test]
    fn holiday_sources_decision_for_public_holiday() {
        let mut sources = HolidaySources::default();
        sources
            .public_holidays
            .insert(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());

        let decision = sources.decision_for(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        assert!(decision.is_holiday);
        assert_eq!(decision.reason, HolidayReason::PublicHoliday);
    }

    #[test]
    fn holiday_sources_decision_for_weekly_holiday() {
        let mut sources = HolidaySources::default();
        sources
            .weekly_holidays
            .insert(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());

        let decision = sources.decision_for(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        assert!(decision.is_holiday);
        assert_eq!(decision.reason, HolidayReason::WeeklyHoliday);
    }

    #[test]
    fn holiday_sources_decision_for_working_day() {
        let sources = HolidaySources::default();

        let decision = sources.decision_for(NaiveDate::from_ymd_opt(2024, 1, 2).unwrap());
        assert!(!decision.is_holiday);
        assert_eq!(decision.reason, HolidayReason::None);
    }

    #[test]
    fn holiday_sources_entry_for_returns_some_for_holiday() {
        let mut sources = HolidaySources::default();
        sources
            .public_holidays
            .insert(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());

        let entry = sources.entry_for(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert!(entry.is_holiday);
        assert_eq!(entry.date, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
    }

    #[test]
    fn holiday_sources_entry_for_returns_none_for_working_day() {
        let sources = HolidaySources::default();

        let entry = sources.entry_for(NaiveDate::from_ymd_opt(2024, 1, 2).unwrap());
        assert!(entry.is_none());
    }

    #[test]
    fn holiday_service_stub_new_creates_stub() {
        let stub = HolidayServiceStub::new(
            vec![NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()],
            vec![],
            vec![],
        );

        let service = stub.service();
        let _service = service;
        let _stub = stub;
    }

    #[test]
    fn holiday_decision_implements_partial_eq() {
        let decision1 = HolidayDecision {
            is_holiday: true,
            reason: HolidayReason::PublicHoliday,
        };
        let decision2 = HolidayDecision {
            is_holiday: true,
            reason: HolidayReason::PublicHoliday,
        };

        assert_eq!(decision1, decision2);
    }

    #[test]
    fn holiday_calendar_entry_has_fields() {
        let entry = HolidayCalendarEntry {
            date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            is_holiday: true,
            reason: HolidayReason::PublicHoliday,
        };

        assert_eq!(entry.date, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        assert!(entry.is_holiday);
        assert_eq!(entry.reason, HolidayReason::PublicHoliday);
    }

    #[test]
    fn expand_weekly_dates_with_enforced_to() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
        let enforced_from = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let enforced_to = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let result = expand_weekly_dates(0, enforced_from, Some(enforced_to), start, end);

        assert!(!result.is_empty());
        assert!(result.len() <= 3);
        assert!(result.iter().all(|d| *d >= start && *d < enforced_to));
    }
}
