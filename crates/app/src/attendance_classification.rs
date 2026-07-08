//! 月次の日次労働時間区分 read use case（T-03 / attendance-calculation-policy.md）。
//!
//! settlement balance 設計（EP-20260703）と同型: DB へ保存しない導出値、
//! 計算不可は tagged status で正当な状態として返す fail-closed read-model。

use std::collections::{BTreeMap, BTreeSet, HashMap};

use async_trait::async_trait;
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime};
use thiserror::Error;
use timekeeper_domain::attendance_classification::{
    build_actual_intervals, classify_days, classify_flex_period, statutory_period_frame_minutes,
    week_start_date, ActualWorkInterval, ClassificationDayInput, WorkRuleParameters,
};

use crate::work_schedules::{ResolvedDayKind, ResolvedWorkday, ScheduleType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthlyClassificationQuery {
    pub user_id: String,
    pub year: i32,
    pub month: u32,
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum MonthlyClassificationError {
    #[error("year must be between 1900 and 9999")]
    InvalidYear,
    #[error("month must be between 1 and 12")]
    InvalidMonth,
    #[error("classification repository error: {0}")]
    Repository(String),
}

/// effective corrections 適用済みの日次実績（打刻）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayActuals {
    pub work_date: NaiveDate,
    pub clock_in_time: Option<NaiveDateTime>,
    pub clock_out_time: Option<NaiveDateTime>,
    pub breaks: Vec<ActualBreak>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActualBreak {
    pub break_start_time: NaiveDateTime,
    pub break_end_time: Option<NaiveDateTime>,
}

/// 就業規則マスタの effective-dated 1 行。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkRuleRow {
    pub valid_from: NaiveDate,
    pub parameters: WorkRuleParameters,
}

#[async_trait]
pub trait ClassificationReadRepository: Send + Sync {
    async fn list_resolved_in_range(
        &self,
        user_id: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<ResolvedWorkday>, MonthlyClassificationError>;

    async fn list_day_actuals_in_range(
        &self,
        user_id: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<DayActuals>, MonthlyClassificationError>;

    /// version の settlement period（契約所定分）。行が無ければ `None`。
    async fn find_settlement_minutes(
        &self,
        version_id: &str,
    ) -> Result<Option<i64>, MonthlyClassificationError>;

    /// `valid_from <= until` の就業規則マスタ行を `valid_from` 昇順で返す。
    async fn list_work_rules_effective_until(
        &self,
        until: NaiveDate,
    ) -> Result<Vec<WorkRuleRow>, MonthlyClassificationError>;
}

/// 計算前に対象窓の resolved workday を materialize する port。
/// `WorkScheduleNotConfigured` の日はスキップして良い（未 resolve のまま残る）。
#[async_trait]
pub trait ClassificationWorkdayMaterializer: Send + Sync {
    async fn materialize(
        &self,
        user_id: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<(), MonthlyClassificationError>;
}

/// 日次区分（応答用。すべて丸めなしの整数分）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassificationDay {
    pub work_date: NaiveDate,
    pub day_kind: ResolvedDayKind,
    pub schedule_type: ScheduleType,
    pub actual_minutes: i64,
    pub scheduled_minutes: i64,
    pub statutory_within_minutes: i64,
    pub statutory_excess_minutes: i64,
    pub legal_holiday_minutes: i64,
    pub night_minutes: i64,
    pub in_progress: bool,
    pub locked: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ClassificationTotals {
    pub actual_minutes: i64,
    pub scheduled_minutes: i64,
    pub statutory_within_minutes: i64,
    pub statutory_excess_minutes: i64,
    pub legal_holiday_minutes: i64,
    pub night_minutes: i64,
}

/// flex 清算期間区分の状態（Decision 8。優先順位は上から）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlexPeriodStatus {
    /// 月内に flex 日が無い（fixed のみ）、または fixed/flex 混在月。
    NotApplicable,
    /// 月内に flex 日を含み、かつ未 resolve の日が残っている。
    UnresolvedDays,
    /// settlement period 値が一致しない複数 version を参照している。
    VersionMixed,
    /// settlement period 行が取得できない（データ不整合）。
    NotConfigured,
    Calculated(FlexPeriodResult),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlexPeriodResult {
    pub contracted_minutes: i64,
    pub statutory_frame_minutes: i64,
    /// flex 日の実労働分の清算期間計（法定休日労働分は除外済み）。
    pub actual_minutes: i64,
    pub scheduled_minutes: i64,
    pub statutory_within_minutes: i64,
    pub statutory_excess_minutes: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonthlyClassification {
    Calculated(CalculatedClassification),
    /// 未 resolve の日に正の実労働分がある（予定情報なしでは区分できない）。
    UnresolvedDays,
    /// 計算窓に就業規則マスタの有効行が無い期間が含まれる。
    WorkRuleNotConfigured,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalculatedClassification {
    pub year: i32,
    pub month: u32,
    pub days: Vec<ClassificationDay>,
    pub totals: ClassificationTotals,
    pub flex_period: FlexPeriodStatus,
}

#[derive(Debug, Clone)]
pub struct GetMonthlyClassification<R, M> {
    repository: R,
    materializer: M,
}

impl<R, M> GetMonthlyClassification<R, M>
where
    R: ClassificationReadRepository,
    M: ClassificationWorkdayMaterializer,
{
    pub fn new(repository: R, materializer: M) -> Self {
        Self {
            repository,
            materializer,
        }
    }

    pub async fn execute(
        &self,
        query: MonthlyClassificationQuery,
    ) -> Result<MonthlyClassification, MonthlyClassificationError> {
        if !(1900..=9999).contains(&query.year) {
            return Err(MonthlyClassificationError::InvalidYear);
        }
        if !(1..=12).contains(&query.month) {
            return Err(MonthlyClassificationError::InvalidMonth);
        }
        let month_start = NaiveDate::from_ymd_opt(query.year, query.month, 1)
            .ok_or(MonthlyClassificationError::InvalidMonth)?;
        let month_end = last_day_of_month(month_start)?;

        let rules = self
            .repository
            .list_work_rules_effective_until(month_end + Duration::days(7))
            .await?;
        let Some(start_params) = params_at(&rules, month_start) else {
            return Ok(MonthlyClassification::WorkRuleNotConfigured);
        };
        let Some(end_params) = params_at(&rules, month_end) else {
            return Ok(MonthlyClassification::WorkRuleNotConfigured);
        };

        // 月跨ぎ週（Decision 5 / 12）: 対象月に重なる週の全日を計算窓にする。
        let window_from = week_start_date(month_start, start_params.week_start_weekday);
        let window_to =
            week_start_date(month_end, end_params.week_start_weekday) + Duration::days(6);
        // 窓の先頭に有効行が無ければ、窓内に未設定期間が存在する（fail-closed）。
        if params_at(&rules, window_from).is_none() {
            return Ok(MonthlyClassification::WorkRuleNotConfigured);
        }

        self.materializer
            .materialize(&query.user_id, window_from, window_to)
            .await?;

        let resolved = self
            .repository
            .list_resolved_in_range(&query.user_id, window_from, window_to)
            .await?;
        let actuals = self
            .repository
            .list_day_actuals_in_range(&query.user_id, window_from, window_to)
            .await?;

        let resolved_by_date: BTreeMap<NaiveDate, ResolvedWorkday> = resolved
            .into_iter()
            .map(|workday| (workday.work_date, workday))
            .collect();
        let actuals_by_date = group_actuals_by_date(actuals);

        // fail-closed: 未 resolve の日に正の実労働分がある場合は区分できない。
        for (work_date, day_actuals) in &actuals_by_date {
            if resolved_by_date.contains_key(work_date) {
                continue;
            }
            if total_minutes(day_actuals) > 0 {
                return Ok(MonthlyClassification::UnresolvedDays);
            }
        }

        let mut inputs = Vec::with_capacity(resolved_by_date.len());
        let mut in_progress_dates = BTreeSet::new();
        for (work_date, workday) in &resolved_by_date {
            let Some(parameters) = params_at(&rules, *work_date) else {
                return Ok(MonthlyClassification::WorkRuleNotConfigured);
            };
            let mut intervals: Vec<ActualWorkInterval> = Vec::new();
            let mut in_progress = false;
            if let Some(day_actuals) = actuals_by_date.get(work_date) {
                for actual in day_actuals {
                    let breaks: Vec<(NaiveDateTime, Option<NaiveDateTime>)> = actual
                        .breaks
                        .iter()
                        .map(|b| (b.break_start_time, b.break_end_time))
                        .collect();
                    intervals.extend(build_actual_intervals(
                        actual.clock_in_time,
                        actual.clock_out_time,
                        &breaks,
                    ));
                    if actual.clock_in_time.is_some() && actual.clock_out_time.is_none() {
                        in_progress = true;
                    }
                }
            }
            if in_progress {
                in_progress_dates.insert(*work_date);
            }
            inputs.push(ClassificationDayInput {
                work_date: *work_date,
                day_kind: workday.day_kind,
                schedule_type: workday.schedule_type,
                expected_work_minutes: i64::from(workday.expected_work_minutes),
                intervals,
                rules: parameters,
            });
        }

        let classified = classify_days(&inputs);

        let mut days = Vec::new();
        let mut totals = ClassificationTotals::default();
        for daily in &classified {
            if daily.work_date < month_start || daily.work_date > month_end {
                continue;
            }
            let workday = resolved_by_date
                .get(&daily.work_date)
                .ok_or_else(|| repository_error("classified day missing resolved workday"))?;
            totals.actual_minutes += daily.actual_minutes;
            totals.scheduled_minutes += daily.scheduled_minutes;
            totals.statutory_within_minutes += daily.statutory_within_minutes;
            totals.statutory_excess_minutes += daily.statutory_excess_minutes;
            totals.legal_holiday_minutes += daily.legal_holiday_minutes;
            totals.night_minutes += daily.night_minutes;
            days.push(ClassificationDay {
                work_date: daily.work_date,
                day_kind: workday.day_kind,
                schedule_type: workday.schedule_type,
                actual_minutes: daily.actual_minutes,
                scheduled_minutes: daily.scheduled_minutes,
                statutory_within_minutes: daily.statutory_within_minutes,
                statutory_excess_minutes: daily.statutory_excess_minutes,
                legal_holiday_minutes: daily.legal_holiday_minutes,
                night_minutes: daily.night_minutes,
                in_progress: in_progress_dates.contains(&daily.work_date),
                locked: workday.locked_at.is_some(),
            });
        }

        let flex_period = self
            .flex_period_status(
                &resolved_by_date,
                &classified,
                month_start,
                month_end,
                start_params,
            )
            .await?;

        Ok(MonthlyClassification::Calculated(
            CalculatedClassification {
                year: query.year,
                month: query.month,
                days,
                totals,
                flex_period,
            },
        ))
    }

    async fn flex_period_status(
        &self,
        resolved_by_date: &BTreeMap<NaiveDate, ResolvedWorkday>,
        classified: &[timekeeper_domain::attendance_classification::DailyClassification],
        month_start: NaiveDate,
        month_end: NaiveDate,
        start_params: WorkRuleParameters,
    ) -> Result<FlexPeriodStatus, MonthlyClassificationError> {
        let month_workdays: Vec<&ResolvedWorkday> = resolved_by_date
            .range(month_start..=month_end)
            .map(|(_, workday)| workday)
            .collect();
        let has_flex = month_workdays
            .iter()
            .any(|workday| workday.schedule_type == ScheduleType::Flex);
        if !has_flex {
            return Ok(FlexPeriodStatus::NotApplicable);
        }

        let days_in_month = i64::from(month_end.day());
        if month_workdays.len() as i64 != days_in_month {
            // settlement balance Decision 8 の流用: flex を含む月の欠損日は fail-closed。
            return Ok(FlexPeriodStatus::UnresolvedDays);
        }
        if month_workdays
            .iter()
            .any(|workday| workday.schedule_type == ScheduleType::Fixed)
        {
            // fixed/flex 混在月の清算期間判定は第一増分では行わない。
            return Ok(FlexPeriodStatus::NotApplicable);
        }

        let version_ids: BTreeSet<&str> = month_workdays
            .iter()
            .map(|workday| workday.work_schedule_version_id.as_str())
            .collect();
        let mut contracted_values = BTreeSet::new();
        let mut missing_settlement = false;
        for version_id in version_ids {
            match self.repository.find_settlement_minutes(version_id).await? {
                Some(value) => {
                    contracted_values.insert(value);
                }
                None => missing_settlement = true,
            }
        }
        if contracted_values.len() > 1 {
            return Ok(FlexPeriodStatus::VersionMixed);
        }
        let Some(contracted_minutes) = (!missing_settlement)
            .then(|| contracted_values.first().copied())
            .flatten()
        else {
            return Ok(FlexPeriodStatus::NotConfigured);
        };

        // 実績計は flex 日の実労働分から法定休日労働分を除外する（二重計上防止）。
        let actual_minutes: i64 = classified
            .iter()
            .filter(|daily| daily.work_date >= month_start && daily.work_date <= month_end)
            .map(|daily| daily.actual_minutes - daily.legal_holiday_minutes)
            .sum();
        let statutory_frame_minutes =
            statutory_period_frame_minutes(start_params.statutory_weekly_minutes, days_in_month);
        let partition =
            classify_flex_period(actual_minutes, contracted_minutes, statutory_frame_minutes);
        Ok(FlexPeriodStatus::Calculated(FlexPeriodResult {
            contracted_minutes,
            statutory_frame_minutes,
            actual_minutes,
            scheduled_minutes: partition.scheduled_minutes,
            statutory_within_minutes: partition.statutory_within_minutes,
            statutory_excess_minutes: partition.statutory_excess_minutes,
        }))
    }
}

fn params_at(rules: &[WorkRuleRow], date: NaiveDate) -> Option<WorkRuleParameters> {
    rules
        .iter()
        .filter(|row| row.valid_from <= date)
        .max_by_key(|row| row.valid_from)
        .map(|row| row.parameters)
}

fn group_actuals_by_date(actuals: Vec<DayActuals>) -> HashMap<NaiveDate, Vec<DayActuals>> {
    let mut grouped: HashMap<NaiveDate, Vec<DayActuals>> = HashMap::new();
    for actual in actuals {
        grouped.entry(actual.work_date).or_default().push(actual);
    }
    grouped
}

fn total_minutes(day_actuals: &[DayActuals]) -> i64 {
    day_actuals
        .iter()
        .map(|actual| {
            let breaks: Vec<(NaiveDateTime, Option<NaiveDateTime>)> = actual
                .breaks
                .iter()
                .map(|b| (b.break_start_time, b.break_end_time))
                .collect();
            build_actual_intervals(actual.clock_in_time, actual.clock_out_time, &breaks)
                .iter()
                .map(ActualWorkInterval::minutes)
                .sum::<i64>()
        })
        .sum()
}

fn last_day_of_month(month_start: NaiveDate) -> Result<NaiveDate, MonthlyClassificationError> {
    let (next_year, next_month) = if month_start.month() == 12 {
        (month_start.year() + 1, 1)
    } else {
        (month_start.year(), month_start.month() + 1)
    };
    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .map(|next_month_start| next_month_start - Duration::days(1))
        .ok_or(MonthlyClassificationError::InvalidYear)
}

fn repository_error(message: &str) -> MonthlyClassificationError {
    MonthlyClassificationError::Repository(message.to_string())
}
