use chrono::{NaiveDate, NaiveTime};
use timekeeper_domain::work_schedules::{
    CoreTimeWindow, DayKind, FlexPolicy, PlannedBreak, PlannedWorkInterval, ScheduleDefinition,
    ScheduleType, ScheduleValidationError, SettlementPeriod, SettlementPeriodUnit, WeekdayRule,
};

fn time(hour: u32, minute: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(hour, minute, 0).expect("valid time")
}

fn working_day(weekday: u8) -> WeekdayRule {
    WeekdayRule {
        weekday,
        day_kind: DayKind::WorkingDay,
        work_intervals: vec![PlannedWorkInterval {
            start_time: time(7, 0),
            start_day_offset: 0,
            end_time: time(22, 0),
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

fn settlement_period(contracted_minutes_per_period: i32) -> SettlementPeriod {
    SettlementPeriod {
        unit: SettlementPeriodUnit::Monthly,
        contracted_minutes_per_period,
    }
}

fn core_time(weekday: u8, start: (u32, u32), end: (u32, u32)) -> CoreTimeWindow {
    core_time_with_offsets(weekday, start, 0, end, 0)
}

fn core_time_with_offsets(
    weekday: u8,
    start: (u32, u32),
    start_day_offset: u8,
    end: (u32, u32),
    end_day_offset: u8,
) -> CoreTimeWindow {
    CoreTimeWindow {
        weekday,
        start_time: time(start.0, start.1),
        start_day_offset,
        end_time: time(end.0, end.1),
        end_day_offset,
    }
}

fn overnight_working_day(weekday: u8, start: (u32, u32), end: (u32, u32)) -> WeekdayRule {
    WeekdayRule {
        weekday,
        day_kind: DayKind::WorkingDay,
        work_intervals: vec![PlannedWorkInterval {
            start_time: time(start.0, start.1),
            start_day_offset: 0,
            end_time: time(end.0, end.1),
            end_day_offset: 1,
        }],
        planned_breaks: Vec::new(),
    }
}

fn flex_definition(core_time_windows: Vec<CoreTimeWindow>) -> ScheduleDefinition {
    ScheduleDefinition {
        effective_from: NaiveDate::from_ymd_opt(2026, 7, 1).expect("date"),
        effective_until: None,
        timezone: "Asia/Tokyo".to_string(),
        workday_boundary: time(5, 0),
        schedule_type: ScheduleType::Flex,
        flex_policy: Some(FlexPolicy {
            settlement_period: settlement_period(9600),
            core_time_windows,
        }),
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

fn flex_definition_with_days(
    days: Vec<WeekdayRule>,
    core_time_windows: Vec<CoreTimeWindow>,
) -> ScheduleDefinition {
    ScheduleDefinition {
        effective_from: NaiveDate::from_ymd_opt(2026, 7, 1).expect("date"),
        effective_until: None,
        timezone: "Asia/Tokyo".to_string(),
        workday_boundary: time(5, 0),
        schedule_type: ScheduleType::Flex,
        flex_policy: Some(FlexPolicy {
            settlement_period: settlement_period(9600),
            core_time_windows,
        }),
        days,
    }
}

fn fixed_definition() -> ScheduleDefinition {
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
fn fixed_schedule_without_flex_policy_remains_valid() {
    let schedule = fixed_definition();
    schedule.validate().expect("valid fixed schedule");
}

#[test]
fn fixed_schedule_rejects_flex_policy() {
    let mut schedule = fixed_definition();
    schedule.flex_policy = Some(FlexPolicy {
        settlement_period: settlement_period(9600),
        core_time_windows: Vec::new(),
    });

    assert_eq!(
        schedule.validate(),
        Err(ScheduleValidationError::FlexPolicyNotAllowedForFixedSchedule)
    );
}

#[test]
fn flex_schedule_requires_flex_policy() {
    let mut schedule = flex_definition(Vec::new());
    schedule.flex_policy = None;

    assert_eq!(
        schedule.validate(),
        Err(ScheduleValidationError::FlexPolicyRequiredForFlexSchedule)
    );
}

#[test]
fn flex_schedule_allows_empty_core_time_windows() {
    let schedule = flex_definition(Vec::new());
    schedule
        .validate()
        .expect("super-flex schedule with no core time is valid");
}

#[test]
fn flex_schedule_accepts_core_time_within_work_interval() {
    let schedule = flex_definition(vec![core_time(1, (10, 0), (15, 0))]);
    schedule
        .validate()
        .expect("core time within flex band is valid");
}

#[test]
fn flex_schedule_rejects_core_time_outside_work_interval() {
    let schedule = flex_definition(vec![core_time(1, (6, 0), (15, 0))]);

    assert_eq!(
        schedule.validate(),
        Err(ScheduleValidationError::CoreTimeOutsideFlexBand { weekday: 1 })
    );
}

#[test]
fn flex_schedule_rejects_inverted_core_time_window() {
    let schedule = flex_definition(vec![core_time(1, (15, 0), (10, 0))]);

    assert_eq!(
        schedule.validate(),
        Err(ScheduleValidationError::InvalidCoreTimeWindow { weekday: 1 })
    );
}

#[test]
fn flex_schedule_rejects_core_time_on_non_working_day() {
    let schedule = flex_definition(vec![core_time(6, (10, 0), (15, 0))]);

    assert_eq!(
        schedule.validate(),
        Err(ScheduleValidationError::CoreTimeOnNonWorkingDay { weekday: 6 })
    );
}

#[test]
fn flex_schedule_rejects_core_time_on_unknown_weekday() {
    let schedule = flex_definition(vec![core_time(9, (10, 0), (15, 0))]);

    assert_eq!(
        schedule.validate(),
        Err(ScheduleValidationError::CoreTimeWeekdayOutOfRange { weekday: 9 })
    );
}

#[test]
fn flex_schedule_rejects_duplicate_core_time_weekday() {
    let schedule = flex_definition(vec![
        core_time(1, (10, 0), (15, 0)),
        core_time(1, (11, 0), (16, 0)),
    ]);

    assert_eq!(
        schedule.validate(),
        Err(ScheduleValidationError::DuplicateCoreTimeWeekday { weekday: 1 })
    );
}

#[test]
fn flex_schedule_rejects_non_positive_settlement_minutes() {
    let mut schedule = flex_definition(Vec::new());
    schedule.flex_policy = Some(FlexPolicy {
        settlement_period: settlement_period(0),
        core_time_windows: Vec::new(),
    });

    assert_eq!(
        schedule.validate(),
        Err(ScheduleValidationError::InvalidSettlementPeriod)
    );
}

#[test]
fn flex_schedule_rejects_settlement_minutes_exceeding_monthly_cap() {
    let mut schedule = flex_definition(Vec::new());
    schedule.flex_policy = Some(FlexPolicy {
        settlement_period: settlement_period(31 * 24 * 60 + 1),
        core_time_windows: Vec::new(),
    });

    assert_eq!(
        schedule.validate(),
        Err(ScheduleValidationError::InvalidSettlementPeriod)
    );
}

#[test]
fn flex_schedule_accepts_settlement_minutes_at_monthly_cap() {
    let mut schedule = flex_definition(Vec::new());
    schedule.flex_policy = Some(FlexPolicy {
        settlement_period: settlement_period(31 * 24 * 60),
        core_time_windows: Vec::new(),
    });

    schedule.validate().expect("cap boundary is inclusive");
}

fn week_with_overnight_monday_and_early_tuesday() -> Vec<WeekdayRule> {
    (1..=7)
        .map(|weekday| match weekday {
            1 => overnight_working_day(1, (22, 0), (7, 0)),
            2 => overnight_working_day(2, (0, 0), (8, 0)),
            weekday if weekday <= 5 => working_day(weekday),
            weekday => non_working_day(weekday),
        })
        .collect()
}

#[test]
fn flex_schedule_accepts_overnight_core_time_within_overnight_band() {
    let schedule = flex_definition_with_days(
        week_with_overnight_monday_and_early_tuesday(),
        vec![core_time_with_offsets(1, (23, 0), 0, (2, 0), 1)],
    );

    schedule
        .validate()
        .expect("core time fully inside the overnight flex band is valid");
}

#[test]
fn flex_schedule_rejects_core_time_with_invalid_day_offset() {
    let schedule = flex_definition_with_days(
        week_with_overnight_monday_and_early_tuesday(),
        vec![core_time_with_offsets(1, (23, 0), 1, (2, 0), 1)],
    );

    assert_eq!(
        schedule.validate(),
        Err(ScheduleValidationError::InvalidDayOffset { weekday: 1 })
    );
}

#[test]
fn flex_schedule_rejects_core_time_windows_overlapping_across_week_boundary() {
    let schedule = flex_definition_with_days(
        week_with_overnight_monday_and_early_tuesday(),
        vec![
            // Spills into Tuesday 00:00-02:00 real time.
            core_time_with_offsets(1, (23, 0), 0, (2, 0), 1),
            // Tuesday 01:00-03:00 real time overlaps the Monday spillover above.
            core_time_with_offsets(2, (1, 0), 0, (3, 0), 0),
        ],
    );

    assert_eq!(
        schedule.validate(),
        Err(ScheduleValidationError::OverlappingCoreTimeWindows {
            weekday_a: 1,
            weekday_b: 2,
        })
    );
}

fn week_with_overnight_sunday_and_early_monday() -> Vec<WeekdayRule> {
    (1..=7)
        .map(|weekday| match weekday {
            1 => overnight_working_day(1, (0, 0), (8, 0)),
            weekday if weekday <= 5 => working_day(weekday),
            7 => overnight_working_day(7, (22, 0), (7, 0)),
            weekday => non_working_day(weekday),
        })
        .collect()
}

#[test]
fn flex_schedule_rejects_core_time_windows_wrapping_past_sunday_into_monday() {
    let schedule = flex_definition_with_days(
        week_with_overnight_sunday_and_early_monday(),
        vec![
            // Sunday 23:00 -> Monday 02:00 real time (wraps past the end of the week).
            core_time_with_offsets(7, (23, 0), 0, (2, 0), 1),
            // Monday 01:00-03:00 real time overlaps the Sunday spillover above.
            core_time_with_offsets(1, (1, 0), 0, (3, 0), 0),
        ],
    );

    assert_eq!(
        schedule.validate(),
        Err(ScheduleValidationError::OverlappingCoreTimeWindows {
            weekday_a: 7,
            weekday_b: 1,
        })
    );
}

#[test]
fn flex_schedule_accepts_adjacent_core_time_windows_that_do_not_overlap() {
    let schedule = flex_definition_with_days(
        week_with_overnight_monday_and_early_tuesday(),
        vec![
            // Spills into Tuesday 00:00-02:00 real time.
            core_time_with_offsets(1, (23, 0), 0, (2, 0), 1),
            // Tuesday 03:00-05:00 real time does not overlap the Monday spillover.
            core_time_with_offsets(2, (3, 0), 0, (5, 0), 0),
        ],
    );

    schedule
        .validate()
        .expect("non-overlapping core time windows across the week boundary are valid");
}
