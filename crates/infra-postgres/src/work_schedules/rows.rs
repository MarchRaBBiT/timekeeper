use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use sqlx::FromRow;
use timekeeper_app::work_schedules::{
    PublicHolidayPolicy, ResolveWorkdayError, ResolvedDayKind, ResolvedWorkday, ScheduleAssignment,
    ScheduleDayRule, ScheduleType, ScheduleVersion, WorkScheduleSource, WorkdayOverride,
    WorkdayOverrideKind,
};
use timekeeper_app::workday_overrides::{StoredWorkdayOverride, WorkdayOverrideError};
use timekeeper_domain::work_schedules::{
    CoreTimeWindow, DayKind, PlannedBreak, PlannedWorkInterval,
};
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub(super) struct ResolvedWorkdayRow {
    pub id: Uuid,
    pub user_id: String,
    pub work_date: NaiveDate,
    pub work_schedule_id: Uuid,
    pub work_schedule_version_id: Uuid,
    pub source: String,
    pub source_id: Uuid,
    pub day_kind: String,
    pub timezone: String,
    pub workday_boundary: NaiveTime,
    pub expected_work_minutes: i32,
    pub schedule_type: String,
    pub resolved_at: DateTime<Utc>,
    pub locked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow)]
pub(super) struct IntervalRow {
    pub start_time: NaiveTime,
    pub start_day_offset: i16,
    pub end_time: NaiveTime,
    pub end_day_offset: i16,
}

#[derive(Debug, FromRow)]
pub(super) struct ResolvedIntervalRow {
    pub resolved_workday_id: Uuid,
    pub start_time: NaiveTime,
    pub start_day_offset: i16,
    pub end_time: NaiveTime,
    pub end_day_offset: i16,
}

#[derive(Debug, FromRow)]
pub(super) struct OverrideRow {
    pub id: Uuid,
    pub kind: String,
    pub work_schedule_id: Option<Uuid>,
}

#[derive(Debug, FromRow)]
pub(super) struct ResolvedBreakRow {
    pub resolved_workday_id: Uuid,
    pub start_time: NaiveTime,
    pub start_day_offset: i16,
    pub end_time: NaiveTime,
    pub end_day_offset: i16,
}

#[derive(Debug, FromRow)]
pub(super) struct CoreTimeWindowRow {
    pub weekday: i16,
    pub start_time: NaiveTime,
    pub start_day_offset: i16,
    pub end_time: NaiveTime,
    pub end_day_offset: i16,
}

#[derive(Debug, FromRow)]
pub(super) struct ResolvedCoreTimeWindowRow {
    pub resolved_workday_id: Uuid,
    pub weekday: i16,
    pub start_time: NaiveTime,
    pub start_day_offset: i16,
    pub end_time: NaiveTime,
    pub end_day_offset: i16,
}

impl TryFrom<CoreTimeWindowRow> for CoreTimeWindow {
    type Error = ResolveWorkdayError;

    fn try_from(row: CoreTimeWindowRow) -> Result<Self, Self::Error> {
        Ok(Self {
            weekday: weekday(row.weekday)?,
            start_time: row.start_time,
            start_day_offset: offset(row.start_day_offset)?,
            end_time: row.end_time,
            end_day_offset: offset(row.end_day_offset)?,
        })
    }
}

fn weekday(value: i16) -> Result<u8, ResolveWorkdayError> {
    u8::try_from(value).map_err(|_| corrupt("weekday", &value.to_string()))
}

#[derive(Debug, FromRow)]
pub(super) struct OverrideRecordRow {
    pub id: Uuid,
    pub user_id: String,
    pub work_date: NaiveDate,
    pub kind: String,
    pub work_schedule_id: Option<Uuid>,
    pub reason: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

fn override_kind(value: &str) -> Result<WorkdayOverrideKind, WorkdayOverrideError> {
    match value {
        "non_working_day" => Ok(WorkdayOverrideKind::NonWorkingDay),
        "use_schedule" => Ok(WorkdayOverrideKind::UseSchedule),
        other => Err(WorkdayOverrideError::Repository(format!(
            "unknown workday override kind: {other}"
        ))),
    }
}

impl TryFrom<OverrideRecordRow> for StoredWorkdayOverride {
    type Error = WorkdayOverrideError;

    fn try_from(row: OverrideRecordRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id.to_string(),
            user_id: row.user_id,
            work_date: row.work_date,
            kind: override_kind(&row.kind)?,
            work_schedule_id: row.work_schedule_id.map(|value| value.to_string()),
            reason: row.reason,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(Debug, FromRow)]
pub(super) struct AssignmentRow {
    pub id: Uuid,
    pub work_schedule_id: Uuid,
}

#[derive(Debug, FromRow)]
pub(super) struct VersionRow {
    pub id: Uuid,
    pub work_schedule_id: Uuid,
    pub timezone: String,
    pub workday_boundary: NaiveTime,
    pub public_holiday_policy: String,
    pub schedule_type: String,
}

#[derive(Debug, FromRow)]
pub(super) struct DayRuleRow {
    pub id: Uuid,
    pub day_kind: String,
    pub expected_work_minutes: i32,
}

pub(super) fn assemble_resolved_workday(
    row: ResolvedWorkdayRow,
    interval_rows: Vec<IntervalRow>,
    break_rows: Vec<IntervalRow>,
    core_time_window_rows: Vec<CoreTimeWindowRow>,
) -> Result<ResolvedWorkday, ResolveWorkdayError> {
    let work_intervals = interval_rows
        .into_iter()
        .map(|value| {
            Ok(PlannedWorkInterval {
                start_time: value.start_time,
                start_day_offset: offset(value.start_day_offset)?,
                end_time: value.end_time,
                end_day_offset: offset(value.end_day_offset)?,
            })
        })
        .collect::<Result<Vec<_>, ResolveWorkdayError>>()?;
    let planned_breaks = break_rows
        .into_iter()
        .map(|value| {
            Ok(PlannedBreak {
                start_time: value.start_time,
                start_day_offset: offset(value.start_day_offset)?,
                end_time: value.end_time,
                end_day_offset: offset(value.end_day_offset)?,
            })
        })
        .collect::<Result<Vec<_>, ResolveWorkdayError>>()?;
    let schedule_type = schedule_type_value(&row.schedule_type)?;
    let day_kind = resolved_day_kind(&row.day_kind)?;
    if schedule_type == ScheduleType::Fixed && !core_time_window_rows.is_empty() {
        return Err(corrupt(
            "resolved workday core time windows",
            "present on a fixed-schedule projection",
        ));
    }
    if day_kind != ResolvedDayKind::ScheduledWorkday && !core_time_window_rows.is_empty() {
        return Err(corrupt(
            "resolved workday core time windows",
            "present on a non-workday projection",
        ));
    }
    let core_time_windows = core_time_window_rows
        .into_iter()
        .map(CoreTimeWindow::try_from)
        .collect::<Result<Vec<_>, ResolveWorkdayError>>()?;
    Ok(ResolvedWorkday {
        id: row.id.to_string(),
        user_id: row.user_id,
        work_date: row.work_date,
        work_schedule_id: row.work_schedule_id.to_string(),
        work_schedule_version_id: row.work_schedule_version_id.to_string(),
        source: source(&row.source)?,
        source_id: row.source_id.to_string(),
        day_kind,
        timezone: row.timezone,
        workday_boundary: row.workday_boundary,
        expected_work_minutes: row.expected_work_minutes,
        work_intervals,
        planned_breaks,
        schedule_type,
        core_time_windows,
        resolved_at: row.resolved_at,
        locked_at: row.locked_at,
    })
}

impl TryFrom<OverrideRow> for WorkdayOverride {
    type Error = ResolveWorkdayError;

    fn try_from(row: OverrideRow) -> Result<Self, Self::Error> {
        let kind = match row.kind.as_str() {
            "non_working_day" => WorkdayOverrideKind::NonWorkingDay,
            "use_schedule" => WorkdayOverrideKind::UseSchedule,
            value => return Err(corrupt("workday override kind", value)),
        };
        Ok(Self {
            id: row.id.to_string(),
            kind,
            work_schedule_id: row.work_schedule_id.map(|value| value.to_string()),
        })
    }
}

impl From<AssignmentRow> for ScheduleAssignment {
    fn from(row: AssignmentRow) -> Self {
        Self {
            id: row.id.to_string(),
            work_schedule_id: row.work_schedule_id.to_string(),
        }
    }
}

impl TryFrom<VersionRow> for ScheduleVersion {
    type Error = ResolveWorkdayError;

    fn try_from(row: VersionRow) -> Result<Self, Self::Error> {
        let public_holiday_policy = match row.public_holiday_policy.as_str() {
            "non_working" => PublicHolidayPolicy::NonWorking,
            "follow_weekly_pattern" => PublicHolidayPolicy::FollowWeeklyPattern,
            value => return Err(corrupt("public holiday policy", value)),
        };
        Ok(Self {
            id: row.id.to_string(),
            work_schedule_id: row.work_schedule_id.to_string(),
            timezone: row.timezone,
            workday_boundary: row.workday_boundary,
            public_holiday_policy,
            schedule_type: schedule_type_value(&row.schedule_type)?,
        })
    }
}

fn schedule_type_value(value: &str) -> Result<ScheduleType, ResolveWorkdayError> {
    match value {
        "fixed" => Ok(ScheduleType::Fixed),
        "flex" => Ok(ScheduleType::Flex),
        other => Err(corrupt("work schedule type", other)),
    }
}

pub(super) fn assemble_day_rule(
    row: DayRuleRow,
    interval_rows: Vec<IntervalRow>,
    break_rows: Vec<IntervalRow>,
) -> Result<ScheduleDayRule, ResolveWorkdayError> {
    let day_kind = match row.day_kind.as_str() {
        "working_day" => DayKind::WorkingDay,
        "non_working_day" => DayKind::NonWorkingDay,
        value => return Err(corrupt("schedule day kind", value)),
    };
    let intervals = interval_rows
        .into_iter()
        .map(|value| {
            Ok(PlannedWorkInterval {
                start_time: value.start_time,
                start_day_offset: offset(value.start_day_offset)?,
                end_time: value.end_time,
                end_day_offset: offset(value.end_day_offset)?,
            })
        })
        .collect::<Result<Vec<_>, ResolveWorkdayError>>()?;
    let breaks = break_rows
        .into_iter()
        .map(|value| {
            Ok(PlannedBreak {
                start_time: value.start_time,
                start_day_offset: offset(value.start_day_offset)?,
                end_time: value.end_time,
                end_day_offset: offset(value.end_day_offset)?,
            })
        })
        .collect::<Result<Vec<_>, ResolveWorkdayError>>()?;
    Ok(ScheduleDayRule {
        day_kind,
        expected_work_minutes: row.expected_work_minutes,
        work_intervals: intervals,
        planned_breaks: breaks,
    })
}

fn source(value: &str) -> Result<WorkScheduleSource, ResolveWorkdayError> {
    match value {
        "override" => Ok(WorkScheduleSource::Override),
        "user" => Ok(WorkScheduleSource::User),
        "department" => Ok(WorkScheduleSource::Department),
        "organization" => Ok(WorkScheduleSource::Organization),
        other => Err(corrupt("work schedule source", other)),
    }
}

fn resolved_day_kind(value: &str) -> Result<ResolvedDayKind, ResolveWorkdayError> {
    match value {
        "scheduled_workday" => Ok(ResolvedDayKind::ScheduledWorkday),
        "scheduled_non_working_day" => Ok(ResolvedDayKind::ScheduledNonWorkingDay),
        "public_holiday" => Ok(ResolvedDayKind::PublicHoliday),
        other => Err(corrupt("resolved day kind", other)),
    }
}

fn offset(value: i16) -> Result<u8, ResolveWorkdayError> {
    u8::try_from(value).map_err(|_| corrupt("day offset", &value.to_string()))
}

fn corrupt(kind: &str, value: &str) -> ResolveWorkdayError {
    ResolveWorkdayError::InvalidScheduleData(format!("unknown {kind}: {value}"))
}
