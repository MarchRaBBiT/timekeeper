use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use chrono::{Datelike, NaiveDate, NaiveDateTime};
use thiserror::Error;
use timekeeper_domain::attendance_classification::{build_actual_intervals, ActualWorkInterval};

use crate::work_schedules::ScheduleType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettlementBalanceQuery {
    pub user_id: String,
    pub year: i32,
    pub month: u32,
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum SettlementBalanceError {
    #[error("year must be between 1900 and 9999")]
    InvalidYear,
    #[error("month must be between 1 and 12")]
    InvalidMonth,
    #[error("settlement balance repository error: {0}")]
    Repository(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettlementWorkday {
    pub work_date: NaiveDate,
    pub version_id: String,
    pub schedule_type: ScheduleType,
    pub locked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettlementActual {
    pub work_date: NaiveDate,
    pub clock_in: Option<NaiveDateTime>,
    pub clock_out: Option<NaiveDateTime>,
    pub breaks: Vec<SettlementBreak>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettlementBreak {
    pub start: NaiveDateTime,
    pub end: Option<NaiveDateTime>,
}

#[async_trait]
pub trait SettlementBalanceReadRepository: Send + Sync {
    async fn list_workdays(
        &self,
        user_id: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<SettlementWorkday>, SettlementBalanceError>;

    async fn list_actuals(
        &self,
        user_id: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<SettlementActual>, SettlementBalanceError>;

    async fn settlement_minutes(
        &self,
        version_id: &str,
    ) -> Result<Option<i64>, SettlementBalanceError>;
}

#[async_trait]
pub trait SettlementWorkdayMaterializer: Send + Sync {
    async fn materialize(
        &self,
        user_id: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<(), SettlementBalanceError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettlementBalanceResult {
    Calculated(CalculatedSettlementBalance),
    UnresolvedDays,
    NotApplicable,
    VersionMixed,
    NotConfigured,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalculatedSettlementBalance {
    pub year: i32,
    pub month: u32,
    pub contracted_minutes: i64,
    pub actual_minutes: i64,
    pub balance_minutes: i64,
    pub days: Vec<SettlementBalanceDay>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettlementBalanceDay {
    pub work_date: NaiveDate,
    pub actual_minutes: i64,
    pub locked: bool,
    pub in_progress: bool,
}

#[derive(Debug, Clone)]
pub struct CalculateSettlementBalance<R, M> {
    repository: R,
    materializer: M,
}

impl<R, M> CalculateSettlementBalance<R, M>
where
    R: SettlementBalanceReadRepository,
    M: SettlementWorkdayMaterializer,
{
    pub fn new(repository: R, materializer: M) -> Self {
        Self {
            repository,
            materializer,
        }
    }

    pub async fn execute(
        &self,
        query: SettlementBalanceQuery,
    ) -> Result<SettlementBalanceResult, SettlementBalanceError> {
        let (month_start, month_end) = month_bounds(query.year, query.month)?;
        self.materializer
            .materialize(&query.user_id, month_start, month_end)
            .await?;
        let workdays = self
            .repository
            .list_workdays(&query.user_id, month_start, month_end)
            .await?;

        if workdays.len() != usize::try_from(month_end.day()).unwrap_or(usize::MAX) {
            return Ok(SettlementBalanceResult::UnresolvedDays);
        }
        if workdays
            .iter()
            .any(|workday| workday.schedule_type == ScheduleType::Fixed)
        {
            return Ok(SettlementBalanceResult::NotApplicable);
        }

        let version_ids: BTreeSet<&str> = workdays
            .iter()
            .map(|workday| workday.version_id.as_str())
            .collect();
        let mut values = BTreeSet::new();
        let mut missing = false;
        for version_id in version_ids {
            match self.repository.settlement_minutes(version_id).await? {
                Some(value) => {
                    values.insert(value);
                }
                None => missing = true,
            }
        }
        if values.len() > 1 {
            return Ok(SettlementBalanceResult::VersionMixed);
        }
        let Some(contracted_minutes) = (!missing).then(|| values.first().copied()).flatten() else {
            return Ok(SettlementBalanceResult::NotConfigured);
        };

        let actuals = self
            .repository
            .list_actuals(&query.user_id, month_start, month_end)
            .await?;
        let mut actuals_by_date: BTreeMap<NaiveDate, Vec<SettlementActual>> = BTreeMap::new();
        for actual in actuals {
            actuals_by_date
                .entry(actual.work_date)
                .or_default()
                .push(actual);
        }

        let mut days = Vec::with_capacity(workdays.len());
        let mut actual_minutes = 0;
        for workday in workdays {
            let daily_actuals = actuals_by_date
                .get(&workday.work_date)
                .map(Vec::as_slice)
                .unwrap_or_default();
            let daily_minutes: i64 = daily_actuals.iter().map(actual_minutes_for_record).sum();
            let in_progress = daily_actuals
                .iter()
                .any(|actual| actual.clock_in.is_some() && actual.clock_out.is_none());
            actual_minutes += daily_minutes;
            days.push(SettlementBalanceDay {
                work_date: workday.work_date,
                actual_minutes: daily_minutes,
                locked: workday.locked,
                in_progress,
            });
        }
        days.sort_by_key(|day| day.work_date);

        Ok(SettlementBalanceResult::Calculated(
            CalculatedSettlementBalance {
                year: query.year,
                month: query.month,
                contracted_minutes,
                actual_minutes,
                balance_minutes: actual_minutes - contracted_minutes,
                days,
            },
        ))
    }
}

fn actual_minutes_for_record(actual: &SettlementActual) -> i64 {
    if actual.clock_out.is_none() {
        return 0;
    }
    let breaks: Vec<(NaiveDateTime, Option<NaiveDateTime>)> = actual
        .breaks
        .iter()
        .map(|period| (period.start, period.end))
        .collect();
    build_actual_intervals(actual.clock_in, actual.clock_out, &breaks)
        .iter()
        .map(ActualWorkInterval::minutes)
        .sum()
}

fn month_bounds(year: i32, month: u32) -> Result<(NaiveDate, NaiveDate), SettlementBalanceError> {
    if !(1900..=9999).contains(&year) {
        return Err(SettlementBalanceError::InvalidYear);
    }
    if !(1..=12).contains(&month) {
        return Err(SettlementBalanceError::InvalidMonth);
    }
    let month_start =
        NaiveDate::from_ymd_opt(year, month, 1).ok_or(SettlementBalanceError::InvalidMonth)?;
    let month_end = (28..=31)
        .rev()
        .find_map(|day| NaiveDate::from_ymd_opt(year, month, day))
        .ok_or(SettlementBalanceError::InvalidMonth)?;
    Ok((month_start, month_end))
}
