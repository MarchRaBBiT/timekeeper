use chrono::{NaiveDate, NaiveTime};
use timekeeper_domain::work_schedules::{
    DayKind, PlannedBreak, PlannedWorkInterval, ScheduleDefinition, ScheduleType,
    ScheduleValidationError, WeekdayRule,
};

fn time(hour: u32, minute: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(hour, minute, 0).expect("valid time")
}

fn working_day(weekday: u8) -> WeekdayRule {
    WeekdayRule {
        weekday,
        day_kind: DayKind::WorkingDay,
        work_intervals: vec![PlannedWorkInterval {
            start_time: time(9, 0),
            start_day_offset: 0,
            end_time: time(18, 0),
            end_day_offset: 0,
        }],
        planned_breaks: vec![PlannedBreak {
            start_time: time(12, 0),
            start_day_offset: 0,
            end_time: time(13, 0),
            end_day_offset: 0,
        }],
    }
}

fn non_working_day(weekday: u8) -> WeekdayRule {
    WeekdayRule {
        weekday,
        day_kind: DayKind::NonWorkingDay,
        work_intervals: Vec::new(),
        planned_breaks: Vec::new(),
    }
}

fn definition() -> ScheduleDefinition {
    ScheduleDefinition {
        effective_from: NaiveDate::from_ymd_opt(2026, 7, 1).expect("date"),
        effective_until: None,
        timezone: "Asia/Tokyo".to_string(),
        workday_boundary: time(5, 0),
        schedule_type: ScheduleType::Fixed,
        flex_policy: None,
        days: (1..=7)
            .map(|weekday| {
                if weekday <= 5 {
                    working_day(weekday)
                } else {
                    non_working_day(weekday)
                }
            })
            .collect(),
    }
}

#[test]
fn validates_fixed_weekly_schedule_and_computes_expected_minutes() {
    let schedule = definition();
    schedule.validate().expect("valid schedule");
    assert_eq!(schedule.days[0].expected_work_minutes(), 480);
}

#[test]
fn accepts_overnight_interval_and_break() {
    let mut schedule = definition();
    schedule.days[0] = WeekdayRule {
        weekday: 1,
        day_kind: DayKind::WorkingDay,
        work_intervals: vec![PlannedWorkInterval {
            start_time: time(22, 0),
            start_day_offset: 0,
            end_time: time(7, 0),
            end_day_offset: 1,
        }],
        planned_breaks: vec![PlannedBreak {
            start_time: time(2, 0),
            start_day_offset: 1,
            end_time: time(3, 0),
            end_day_offset: 1,
        }],
    };

    schedule.validate().expect("valid night schedule");
    assert_eq!(schedule.days[0].expected_work_minutes(), 480);
}

#[test]
fn rejects_duplicate_or_missing_weekdays() {
    let mut schedule = definition();
    schedule.days[6].weekday = 1;

    assert_eq!(
        schedule.validate(),
        Err(ScheduleValidationError::WeekdaysMustCoverOneThroughSeven)
    );
}

#[test]
fn rejects_break_outside_work_interval() {
    let mut schedule = definition();
    schedule.days[0].planned_breaks[0] = PlannedBreak {
        start_time: time(18, 0),
        start_day_offset: 0,
        end_time: time(19, 0),
        end_day_offset: 0,
    };

    assert_eq!(
        schedule.validate(),
        Err(ScheduleValidationError::BreakOutsideWorkInterval { weekday: 1 })
    );
}

#[test]
fn rejects_non_working_day_with_intervals() {
    let mut schedule = definition();
    schedule.days[5].work_intervals = working_day(6).work_intervals;

    assert_eq!(
        schedule.validate(),
        Err(ScheduleValidationError::NonWorkingDayHasIntervals { weekday: 6 })
    );
}

#[test]
fn rejects_inverted_effective_period_and_unknown_timezone() {
    let mut schedule = definition();
    schedule.effective_until = Some(schedule.effective_from);
    assert_eq!(
        schedule.validate(),
        Err(ScheduleValidationError::InvalidEffectivePeriod)
    );

    let mut schedule = definition();
    schedule.timezone = "Mars/Olympus".to_string();
    assert_eq!(
        schedule.validate(),
        Err(ScheduleValidationError::InvalidTimezone)
    );
}

#[test]
fn rejects_unbounded_planned_break_input() {
    let mut schedule = definition();
    schedule.days[0].planned_breaks = (0..17)
        .map(|_| PlannedBreak {
            start_time: time(12, 0),
            start_day_offset: 0,
            end_time: time(12, 1),
            end_day_offset: 0,
        })
        .collect();

    assert_eq!(
        schedule.validate(),
        Err(ScheduleValidationError::TooManyBreaks { weekday: 1 })
    );
}
