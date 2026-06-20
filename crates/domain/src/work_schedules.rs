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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleDefinition {
    pub effective_from: NaiveDate,
    pub effective_until: Option<NaiveDate>,
    pub timezone: String,
    pub workday_boundary: NaiveTime,
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
        Ok(())
    }
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
}
