use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use sqlx::FromRow;
use timekeeper_contract::work_schedules::{
    AssignmentTarget, CoreTimeWindowResponse, DayKind, FlexPolicyResponse, PlannedBreakResponse,
    PlannedWorkIntervalResponse, PublicHolidayPolicy, SettlementPeriodResponse,
    SettlementPeriodUnit, WeekdayRuleResponse, WorkScheduleAssignmentResponse,
    WorkScheduleResponse, WorkScheduleStatus, WorkScheduleType, WorkScheduleVersionResponse,
    WorkScheduleVersionStatus, WorkScheduleVersionSummary,
};
use uuid::Uuid;

use super::{RepositoryResult, WorkScheduleRepositoryError};

#[derive(Debug, FromRow)]
pub(super) struct WorkScheduleRow {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<WorkScheduleRow> for WorkScheduleResponse {
    type Error = WorkScheduleRepositoryError;

    fn try_from(row: WorkScheduleRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id.to_string(),
            code: row.code,
            name: row.name,
            description: row.description,
            status: schedule_status(&row.status)?,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(Debug, FromRow)]
pub(super) struct VersionSummaryRow {
    pub id: Uuid,
    pub version_number: i32,
    pub status: String,
    pub effective_from: NaiveDate,
    pub effective_until: Option<NaiveDate>,
    pub revision: i32,
    pub published_at: Option<DateTime<Utc>>,
}

impl TryFrom<VersionSummaryRow> for WorkScheduleVersionSummary {
    type Error = WorkScheduleRepositoryError;

    fn try_from(row: VersionSummaryRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id.to_string(),
            version_number: row.version_number,
            status: version_status(&row.status)?,
            effective_from: row.effective_from,
            effective_until: row.effective_until,
            revision: row.revision,
            published_at: row.published_at,
        })
    }
}

#[derive(Debug, FromRow)]
pub(super) struct VersionRow {
    pub id: Uuid,
    pub work_schedule_id: Uuid,
    pub version_number: i32,
    pub status: String,
    pub effective_from: NaiveDate,
    pub effective_until: Option<NaiveDate>,
    pub timezone: String,
    pub workday_boundary: NaiveTime,
    pub public_holiday_policy: String,
    pub late_grace_minutes: i32,
    pub early_leave_grace_minutes: i32,
    pub schedule_type: String,
    pub revision: i32,
    pub published_by: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub(super) struct SettlementPeriodRow {
    pub unit: String,
    pub contracted_minutes_per_period: i32,
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
pub(super) struct DayRuleRow {
    pub id: Uuid,
    pub weekday: i16,
    pub day_kind: String,
    pub expected_work_minutes: i32,
}

#[derive(Debug, FromRow)]
pub(super) struct IntervalRow {
    pub day_rule_id: Uuid,
    pub sequence: i32,
    pub start_time: NaiveTime,
    pub start_day_offset: i16,
    pub end_time: NaiveTime,
    pub end_day_offset: i16,
}

#[derive(Debug, FromRow)]
pub(super) struct AssignmentRow {
    pub id: Uuid,
    pub work_schedule_id: Uuid,
    pub user_id: Option<String>,
    pub department_id: Option<String>,
    pub is_org_default: bool,
    pub valid_from: NaiveDate,
    pub valid_until: Option<NaiveDate>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<AssignmentRow> for WorkScheduleAssignmentResponse {
    type Error = WorkScheduleRepositoryError;

    fn try_from(row: AssignmentRow) -> Result<Self, Self::Error> {
        let target = match (row.is_org_default, row.department_id, row.user_id) {
            (true, None, None) => AssignmentTarget::Organization,
            (false, Some(department_id), None) => AssignmentTarget::Department { department_id },
            (false, None, Some(user_id)) => AssignmentTarget::User { user_id },
            _ => {
                return Err(WorkScheduleRepositoryError::CorruptData(
                    "assignment target is ambiguous".to_string(),
                ));
            }
        };
        Ok(Self {
            id: row.id.to_string(),
            work_schedule_id: row.work_schedule_id.to_string(),
            target,
            valid_from: row.valid_from,
            valid_until: row.valid_until,
            created_by: row.created_by,
            created_at: row.created_at,
        })
    }
}

pub(super) fn assemble_version(
    version: VersionRow,
    day_rows: Vec<DayRuleRow>,
    interval_rows: Vec<IntervalRow>,
    break_rows: Vec<IntervalRow>,
    settlement_period_row: Option<SettlementPeriodRow>,
    core_time_window_rows: Vec<CoreTimeWindowRow>,
) -> RepositoryResult<WorkScheduleVersionResponse> {
    let mut intervals: HashMap<Uuid, Vec<PlannedWorkIntervalResponse>> = HashMap::new();
    for row in interval_rows {
        intervals
            .entry(row.day_rule_id)
            .or_default()
            .push(PlannedWorkIntervalResponse {
                sequence: row.sequence,
                start_time: row.start_time,
                start_day_offset: row.start_day_offset,
                end_time: row.end_time,
                end_day_offset: row.end_day_offset,
            });
    }
    let mut breaks: HashMap<Uuid, Vec<PlannedBreakResponse>> = HashMap::new();
    for row in break_rows {
        breaks
            .entry(row.day_rule_id)
            .or_default()
            .push(PlannedBreakResponse {
                sequence: row.sequence,
                start_time: row.start_time,
                start_day_offset: row.start_day_offset,
                end_time: row.end_time,
                end_day_offset: row.end_day_offset,
            });
    }
    let days = day_rows
        .into_iter()
        .map(|row| {
            Ok(WeekdayRuleResponse {
                weekday: row.weekday,
                day_kind: day_kind(&row.day_kind)?,
                expected_work_minutes: row.expected_work_minutes,
                work_intervals: intervals.remove(&row.id).unwrap_or_default(),
                planned_breaks: breaks.remove(&row.id).unwrap_or_default(),
            })
        })
        .collect::<RepositoryResult<Vec<_>>>()?;

    let version_schedule_type = schedule_type(&version.schedule_type)?;
    let flex_policy = match (version_schedule_type, settlement_period_row) {
        (WorkScheduleType::Fixed, None) => None,
        (WorkScheduleType::Fixed, Some(_)) => {
            return Err(WorkScheduleRepositoryError::CorruptData(
                "fixed work schedule version has a stored settlement period".to_string(),
            ));
        }
        (WorkScheduleType::Flex, None) => {
            return Err(WorkScheduleRepositoryError::CorruptData(
                "flex work schedule version is missing its settlement period".to_string(),
            ));
        }
        (WorkScheduleType::Flex, Some(settlement_period)) => Some(FlexPolicyResponse {
            settlement_period: SettlementPeriodResponse {
                unit: settlement_period_unit(&settlement_period.unit)?,
                contracted_minutes_per_period: settlement_period.contracted_minutes_per_period,
            },
            core_time_windows: core_time_window_rows
                .into_iter()
                .map(|row| CoreTimeWindowResponse {
                    weekday: row.weekday,
                    start_time: row.start_time,
                    start_day_offset: row.start_day_offset,
                    end_time: row.end_time,
                    end_day_offset: row.end_day_offset,
                })
                .collect(),
        }),
    };

    Ok(WorkScheduleVersionResponse {
        id: version.id.to_string(),
        work_schedule_id: version.work_schedule_id.to_string(),
        version_number: version.version_number,
        status: version_status(&version.status)?,
        effective_from: version.effective_from,
        effective_until: version.effective_until,
        timezone: version.timezone,
        workday_boundary: version.workday_boundary,
        public_holiday_policy: holiday_policy(&version.public_holiday_policy)?,
        late_grace_minutes: version.late_grace_minutes,
        early_leave_grace_minutes: version.early_leave_grace_minutes,
        schedule_type: version_schedule_type,
        flex_policy,
        revision: version.revision,
        published_by: version.published_by,
        published_at: version.published_at,
        created_at: version.created_at,
        updated_at: version.updated_at,
        days,
    })
}

fn schedule_status(value: &str) -> RepositoryResult<WorkScheduleStatus> {
    match value {
        "active" => Ok(WorkScheduleStatus::Active),
        "retired" => Ok(WorkScheduleStatus::Retired),
        other => Err(corrupt_enum("work schedule status", other)),
    }
}

fn version_status(value: &str) -> RepositoryResult<WorkScheduleVersionStatus> {
    match value {
        "draft" => Ok(WorkScheduleVersionStatus::Draft),
        "published" => Ok(WorkScheduleVersionStatus::Published),
        "cancelled" => Ok(WorkScheduleVersionStatus::Cancelled),
        other => Err(corrupt_enum("work schedule version status", other)),
    }
}

fn holiday_policy(value: &str) -> RepositoryResult<PublicHolidayPolicy> {
    match value {
        "non_working" => Ok(PublicHolidayPolicy::NonWorking),
        "follow_weekly_pattern" => Ok(PublicHolidayPolicy::FollowWeeklyPattern),
        other => Err(corrupt_enum("public holiday policy", other)),
    }
}

fn schedule_type(value: &str) -> RepositoryResult<WorkScheduleType> {
    match value {
        "fixed" => Ok(WorkScheduleType::Fixed),
        "flex" => Ok(WorkScheduleType::Flex),
        other => Err(corrupt_enum("work schedule type", other)),
    }
}

fn settlement_period_unit(value: &str) -> RepositoryResult<SettlementPeriodUnit> {
    match value {
        "monthly" => Ok(SettlementPeriodUnit::Monthly),
        other => Err(corrupt_enum("settlement period unit", other)),
    }
}

fn day_kind(value: &str) -> RepositoryResult<DayKind> {
    match value {
        "working_day" => Ok(DayKind::WorkingDay),
        "non_working_day" => Ok(DayKind::NonWorkingDay),
        other => Err(corrupt_enum("day kind", other)),
    }
}

fn corrupt_enum(kind: &str, value: &str) -> WorkScheduleRepositoryError {
    WorkScheduleRepositoryError::CorruptData(format!("unknown {kind}: {value}"))
}
