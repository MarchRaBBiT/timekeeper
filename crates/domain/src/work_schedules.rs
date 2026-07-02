use std::{collections::BTreeSet, str::FromStr};

use chrono::{NaiveDate, NaiveTime, Timelike};
use chrono_tz::Tz;

const MAX_PLANNED_BREAKS_PER_DAY: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DayKind {
    WorkingDay,
    NonWorkingDay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicHolidayPolicy {
    NonWorking,
    FollowWeeklyPattern,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkScheduleSource {
    Override,
    User,
    Department,
    Organization,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedDayKind {
    ScheduledWorkday,
    ScheduledNonWorkingDay,
    PublicHoliday,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedWorkInterval {
    pub start_time: NaiveTime,
    pub start_day_offset: u8,
    pub end_time: NaiveTime,
    pub end_day_offset: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedBreak {
    pub start_time: NaiveTime,
    pub start_day_offset: u8,
    pub end_time: NaiveTime,
    pub end_day_offset: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeekdayRule {
    pub weekday: u8,
    pub day_kind: DayKind,
    pub work_intervals: Vec<PlannedWorkInterval>,
    pub planned_breaks: Vec<PlannedBreak>,
}

impl WeekdayRule {
    pub fn expected_work_minutes(&self) -> i32 {
        let work_minutes: i64 = self
            .work_intervals
            .iter()
            .map(|interval| {
                interval_minutes(
                    interval.start_time,
                    interval.start_day_offset,
                    interval.end_time,
                    interval.end_day_offset,
                )
            })
            .sum();
        let break_minutes: i64 = self
            .planned_breaks
            .iter()
            .map(|planned_break| {
                interval_minutes(
                    planned_break.start_time,
                    planned_break.start_day_offset,
                    planned_break.end_time,
                    planned_break.end_day_offset,
                )
            })
            .sum();
        i32::try_from((work_minutes - break_minutes).max(0)).unwrap_or(i32::MAX)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleType {
    Fixed,
    Flex,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreTimeWindow {
    pub weekday: u8,
    pub start_time: NaiveTime,
    pub start_day_offset: u8,
    pub end_time: NaiveTime,
    pub end_day_offset: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettlementPeriodUnit {
    Monthly,
}

impl SettlementPeriodUnit {
    fn max_minutes(self) -> i32 {
        match self {
            SettlementPeriodUnit::Monthly => 31 * 24 * 60,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettlementPeriod {
    pub unit: SettlementPeriodUnit,
    pub contracted_minutes_per_period: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlexPolicy {
    pub settlement_period: SettlementPeriod,
    pub core_time_windows: Vec<CoreTimeWindow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleDefinition {
    pub effective_from: NaiveDate,
    pub effective_until: Option<NaiveDate>,
    pub timezone: String,
    pub workday_boundary: NaiveTime,
    pub schedule_type: ScheduleType,
    pub flex_policy: Option<FlexPolicy>,
    pub days: Vec<WeekdayRule>,
}

impl ScheduleDefinition {
    pub fn validate(&self) -> Result<(), ScheduleValidationError> {
        if self
            .effective_until
            .is_some_and(|until| until <= self.effective_from)
        {
            return Err(ScheduleValidationError::InvalidEffectivePeriod);
        }
        if Tz::from_str(&self.timezone).is_err() {
            return Err(ScheduleValidationError::InvalidTimezone);
        }

        let weekdays: BTreeSet<u8> = self.days.iter().map(|day| day.weekday).collect();
        if self.days.len() != 7 || weekdays != BTreeSet::from([1, 2, 3, 4, 5, 6, 7]) {
            return Err(ScheduleValidationError::WeekdaysMustCoverOneThroughSeven);
        }

        for day in &self.days {
            validate_day(day)?;
        }

        self.validate_flex_policy()?;
        Ok(())
    }

    fn validate_flex_policy(&self) -> Result<(), ScheduleValidationError> {
        match (self.schedule_type, &self.flex_policy) {
            (ScheduleType::Fixed, None) => Ok(()),
            (ScheduleType::Fixed, Some(_)) => {
                Err(ScheduleValidationError::FlexPolicyNotAllowedForFixedSchedule)
            }
            (ScheduleType::Flex, None) => {
                Err(ScheduleValidationError::FlexPolicyRequiredForFlexSchedule)
            }
            (ScheduleType::Flex, Some(policy)) => {
                let minutes = policy.settlement_period.contracted_minutes_per_period;
                if minutes <= 0 || minutes > policy.settlement_period.unit.max_minutes() {
                    return Err(ScheduleValidationError::InvalidSettlementPeriod);
                }

                let mut seen_weekdays = BTreeSet::new();
                let mut weekly_ranges: Vec<(u8, i64, i64)> = Vec::new();
                for window in &policy.core_time_windows {
                    if !seen_weekdays.insert(window.weekday) {
                        return Err(ScheduleValidationError::DuplicateCoreTimeWeekday {
                            weekday: window.weekday,
                        });
                    }
                    let day = self
                        .days
                        .iter()
                        .find(|day| day.weekday == window.weekday)
                        .ok_or(ScheduleValidationError::CoreTimeWeekdayOutOfRange {
                            weekday: window.weekday,
                        })?;
                    if day.day_kind != DayKind::WorkingDay {
                        return Err(ScheduleValidationError::CoreTimeOnNonWorkingDay {
                            weekday: window.weekday,
                        });
                    }
                    validate_work_offsets(
                        window.start_day_offset,
                        window.end_day_offset,
                        window.weekday,
                    )?;
                    let core_start = minute_index(window.start_time, window.start_day_offset);
                    let core_end = minute_index(window.end_time, window.end_day_offset);
                    if core_start >= core_end {
                        return Err(ScheduleValidationError::InvalidCoreTimeWindow {
                            weekday: window.weekday,
                        });
                    }
                    let contains_core_time = day.work_intervals.iter().any(|interval| {
                        let work_start =
                            minute_index(interval.start_time, interval.start_day_offset);
                        let work_end = minute_index(interval.end_time, interval.end_day_offset);
                        core_start >= work_start && core_end <= work_end
                    });
                    if !contains_core_time {
                        return Err(ScheduleValidationError::CoreTimeOutsideFlexBand {
                            weekday: window.weekday,
                        });
                    }

                    let week_start = i64::from(window.weekday - 1) * MINUTES_PER_DAY + core_start;
                    let week_end = week_start + (core_end - core_start);
                    weekly_ranges.push((window.weekday, week_start, week_end));
                }

                for i in 0..weekly_ranges.len() {
                    for j in (i + 1)..weekly_ranges.len() {
                        let (weekday_a, start_a, end_a) = weekly_ranges[i];
                        let (weekday_b, start_b, end_b) = weekly_ranges[j];
                        if weekly_ranges_overlap(start_a, end_a, start_b, end_b) {
                            return Err(ScheduleValidationError::OverlappingCoreTimeWindows {
                                weekday_a,
                                weekday_b,
                            });
                        }
                    }
                }
                Ok(())
            }
        }
    }
}

const MINUTES_PER_WEEK: i64 = 7 * 24 * 60;
const MINUTES_PER_DAY: i64 = 24 * 60;

fn weekly_ranges_overlap(start_a: i64, end_a: i64, start_b: i64, end_b: i64) -> bool {
    [-MINUTES_PER_WEEK, 0, MINUTES_PER_WEEK]
        .iter()
        .any(|shift| start_a < end_b + shift && start_b + shift < end_a)
}

fn validate_day(day: &WeekdayRule) -> Result<(), ScheduleValidationError> {
    match day.day_kind {
        DayKind::NonWorkingDay => {
            if !day.work_intervals.is_empty() || !day.planned_breaks.is_empty() {
                return Err(ScheduleValidationError::NonWorkingDayHasIntervals {
                    weekday: day.weekday,
                });
            }
        }
        DayKind::WorkingDay => {
            if day.work_intervals.len() != 1 {
                return Err(ScheduleValidationError::WorkingDayRequiresSingleInterval {
                    weekday: day.weekday,
                });
            }
            let work = &day.work_intervals[0];
            if day.planned_breaks.len() > MAX_PLANNED_BREAKS_PER_DAY {
                return Err(ScheduleValidationError::TooManyBreaks {
                    weekday: day.weekday,
                });
            }
            validate_work_offsets(work.start_day_offset, work.end_day_offset, day.weekday)?;
            let work_start = minute_index(work.start_time, work.start_day_offset);
            let work_end = minute_index(work.end_time, work.end_day_offset);
            if work_start >= work_end {
                return Err(ScheduleValidationError::InvalidWorkInterval {
                    weekday: day.weekday,
                });
            }

            let mut break_ranges = Vec::with_capacity(day.planned_breaks.len());
            for planned_break in &day.planned_breaks {
                validate_break_offsets(
                    planned_break.start_day_offset,
                    planned_break.end_day_offset,
                    day.weekday,
                )?;
                let break_start =
                    minute_index(planned_break.start_time, planned_break.start_day_offset);
                let break_end = minute_index(planned_break.end_time, planned_break.end_day_offset);
                if break_start >= break_end || break_start < work_start || break_end > work_end {
                    return Err(ScheduleValidationError::BreakOutsideWorkInterval {
                        weekday: day.weekday,
                    });
                }
                break_ranges.push((break_start, break_end));
            }
            break_ranges.sort_unstable();
            if break_ranges.windows(2).any(|pair| pair[0].1 > pair[1].0) {
                return Err(ScheduleValidationError::OverlappingBreaks {
                    weekday: day.weekday,
                });
            }
        }
    }
    Ok(())
}

fn validate_work_offsets(
    start_day_offset: u8,
    end_day_offset: u8,
    weekday: u8,
) -> Result<(), ScheduleValidationError> {
    if start_day_offset != 0 || end_day_offset > 1 || end_day_offset < start_day_offset {
        return Err(ScheduleValidationError::InvalidDayOffset { weekday });
    }
    Ok(())
}

fn validate_break_offsets(
    start_day_offset: u8,
    end_day_offset: u8,
    weekday: u8,
) -> Result<(), ScheduleValidationError> {
    if start_day_offset > 1 || end_day_offset > 1 || end_day_offset < start_day_offset {
        return Err(ScheduleValidationError::InvalidDayOffset { weekday });
    }
    Ok(())
}

fn minute_index(time: NaiveTime, day_offset: u8) -> i64 {
    i64::from(day_offset) * 24 * 60 + i64::from(time.hour()) * 60 + i64::from(time.minute())
}

fn interval_minutes(
    start_time: NaiveTime,
    start_day_offset: u8,
    end_time: NaiveTime,
    end_day_offset: u8,
) -> i64 {
    minute_index(end_time, end_day_offset) - minute_index(start_time, start_day_offset)
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ScheduleValidationError {
    #[error("effective_until must be later than effective_from")]
    InvalidEffectivePeriod,
    #[error("timezone must be a valid IANA timezone")]
    InvalidTimezone,
    #[error("weekday rules must cover ISO weekdays 1 through 7 exactly once")]
    WeekdaysMustCoverOneThroughSeven,
    #[error("non-working weekday {weekday} cannot contain work or break intervals")]
    NonWorkingDayHasIntervals { weekday: u8 },
    #[error("working weekday {weekday} must contain exactly one work interval")]
    WorkingDayRequiresSingleInterval { weekday: u8 },
    #[error("weekday {weekday} has an invalid work interval")]
    InvalidWorkInterval { weekday: u8 },
    #[error("weekday {weekday} has an invalid day offset")]
    InvalidDayOffset { weekday: u8 },
    #[error("weekday {weekday} contains a break outside its work interval")]
    BreakOutsideWorkInterval { weekday: u8 },
    #[error("weekday {weekday} contains overlapping breaks")]
    OverlappingBreaks { weekday: u8 },
    #[error("weekday {weekday} contains too many planned breaks")]
    TooManyBreaks { weekday: u8 },
    #[error("flex_policy is not allowed for a fixed schedule")]
    FlexPolicyNotAllowedForFixedSchedule,
    #[error("flex_policy is required for a flex schedule")]
    FlexPolicyRequiredForFlexSchedule,
    #[error("settlement_period contracted_minutes_per_period is out of range")]
    InvalidSettlementPeriod,
    #[error("core time window for weekday {weekday} references a weekday outside the schedule")]
    CoreTimeWeekdayOutOfRange { weekday: u8 },
    #[error("core time window for weekday {weekday} falls on a non-working day")]
    CoreTimeOnNonWorkingDay { weekday: u8 },
    #[error("core time window for weekday {weekday} is invalid")]
    InvalidCoreTimeWindow { weekday: u8 },
    #[error("core time window for weekday {weekday} extends outside the flexible work band")]
    CoreTimeOutsideFlexBand { weekday: u8 },
    #[error("weekday {weekday} has more than one core time window")]
    DuplicateCoreTimeWeekday { weekday: u8 },
    #[error(
        "core time windows for weekday {weekday_a} and weekday {weekday_b} overlap in real time"
    )]
    OverlappingCoreTimeWindows { weekday_a: u8, weekday_b: u8 },
}
