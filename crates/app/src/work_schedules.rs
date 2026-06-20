use async_trait::async_trait;
use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, Utc};
use thiserror::Error;
use timekeeper_domain::work_schedules::{DayKind, PlannedBreak, PlannedWorkInterval};

pub use timekeeper_domain::work_schedules::{
    PublicHolidayPolicy, ResolvedDayKind, WorkScheduleSource,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveWorkdayCommand {
    pub user_id: String,
    pub work_date: NaiveDate,
    pub resolved_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AssignmentTarget {
    User(String),
    Department(String),
    Organization,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleAssignment {
    pub id: String,
    pub work_schedule_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkdayOverrideKind {
    NonWorkingDay,
    UseSchedule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkdayOverride {
    pub id: String,
    pub kind: WorkdayOverrideKind,
    pub work_schedule_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleVersion {
    pub id: String,
    pub work_schedule_id: String,
    pub timezone: String,
    pub workday_boundary: NaiveTime,
    pub public_holiday_policy: PublicHolidayPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleDayRule {
    pub day_kind: DayKind,
    pub expected_work_minutes: i32,
    pub work_intervals: Vec<PlannedWorkInterval>,
    pub planned_breaks: Vec<PlannedBreak>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewResolvedWorkday {
    pub user_id: String,
    pub work_date: NaiveDate,
    pub work_schedule_id: String,
    pub work_schedule_version_id: String,
    pub source: WorkScheduleSource,
    pub source_id: String,
    pub day_kind: ResolvedDayKind,
    pub timezone: String,
    pub workday_boundary: NaiveTime,
    pub expected_work_minutes: i32,
    pub work_intervals: Vec<PlannedWorkInterval>,
    pub planned_breaks: Vec<PlannedBreak>,
    pub resolved_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedWorkday {
    pub id: String,
    pub user_id: String,
    pub work_date: NaiveDate,
    pub work_schedule_id: String,
    pub work_schedule_version_id: String,
    pub source: WorkScheduleSource,
    pub source_id: String,
    pub day_kind: ResolvedDayKind,
    pub timezone: String,
    pub workday_boundary: NaiveTime,
    pub expected_work_minutes: i32,
    pub work_intervals: Vec<PlannedWorkInterval>,
    pub planned_breaks: Vec<PlannedBreak>,
    pub resolved_at: DateTime<Utc>,
    pub locked_at: Option<DateTime<Utc>>,
}

impl ResolvedWorkday {
    pub fn from_new(id: String, projection: NewResolvedWorkday) -> Self {
        Self {
            id,
            user_id: projection.user_id,
            work_date: projection.work_date,
            work_schedule_id: projection.work_schedule_id,
            work_schedule_version_id: projection.work_schedule_version_id,
            source: projection.source,
            source_id: projection.source_id,
            day_kind: projection.day_kind,
            timezone: projection.timezone,
            workday_boundary: projection.workday_boundary,
            expected_work_minutes: projection.expected_work_minutes,
            work_intervals: projection.work_intervals,
            planned_breaks: projection.planned_breaks,
            resolved_at: projection.resolved_at,
            locked_at: None,
        }
    }
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ResolveWorkdayError {
    #[error("work schedule is not configured for the requested day")]
    WorkScheduleNotConfigured,
    #[error("work schedule data is invalid: {0}")]
    InvalidScheduleData(String),
    #[error("workday resolution repository error: {0}")]
    Repository(String),
}

#[async_trait]
pub trait WorkdayResolutionRepository: Send + Sync {
    async fn find_resolved(
        &self,
        user_id: &str,
        work_date: NaiveDate,
    ) -> Result<Option<ResolvedWorkday>, ResolveWorkdayError>;

    async fn find_override(
        &self,
        user_id: &str,
        work_date: NaiveDate,
    ) -> Result<Option<WorkdayOverride>, ResolveWorkdayError>;

    async fn find_assignment(
        &self,
        target: &AssignmentTarget,
        work_date: NaiveDate,
    ) -> Result<Option<ScheduleAssignment>, ResolveWorkdayError>;

    async fn find_published_version(
        &self,
        work_schedule_id: &str,
        work_date: NaiveDate,
    ) -> Result<Option<ScheduleVersion>, ResolveWorkdayError>;

    async fn find_day_rule(
        &self,
        version_id: &str,
        weekday: u8,
    ) -> Result<Option<ScheduleDayRule>, ResolveWorkdayError>;

    async fn save_projection(
        &self,
        projection: NewResolvedWorkday,
    ) -> Result<ResolvedWorkday, ResolveWorkdayError>;
}

#[async_trait]
pub trait OrganizationHierarchy: Send + Sync {
    async fn department_lineage(&self, user_id: &str) -> Result<Vec<String>, ResolveWorkdayError>;
}

#[async_trait]
pub trait WorkdayHolidayCalendar: Send + Sync {
    async fn is_public_holiday(&self, work_date: NaiveDate) -> Result<bool, ResolveWorkdayError>;
}

#[derive(Debug, Clone)]
pub struct ResolveWorkday<R, O, H> {
    repository: R,
    organization_hierarchy: O,
    holiday_calendar: H,
}

impl<R, O, H> ResolveWorkday<R, O, H>
where
    R: WorkdayResolutionRepository,
    O: OrganizationHierarchy,
    H: WorkdayHolidayCalendar,
{
    pub fn new(repository: R, organization_hierarchy: O, holiday_calendar: H) -> Self {
        Self {
            repository,
            organization_hierarchy,
            holiday_calendar,
        }
    }

    pub fn repository(&self) -> &R {
        &self.repository
    }

    pub async fn execute(
        &self,
        command: ResolveWorkdayCommand,
    ) -> Result<ResolvedWorkday, ResolveWorkdayError> {
        if let Some(existing) = self
            .repository
            .find_resolved(&command.user_id, command.work_date)
            .await?
        {
            if existing.locked_at.is_some() {
                return Ok(existing);
            }
        }

        let day_override = self
            .repository
            .find_override(&command.user_id, command.work_date)
            .await?;
        let assignment = match day_override.as_ref() {
            Some(WorkdayOverride {
                kind: WorkdayOverrideKind::UseSchedule,
                work_schedule_id: Some(work_schedule_id),
                ..
            }) => SelectedSchedule {
                work_schedule_id: work_schedule_id.clone(),
                source: WorkScheduleSource::Override,
                source_id: day_override
                    .as_ref()
                    .map(|value| value.id.clone())
                    .ok_or_else(|| invalid_data("override disappeared"))?,
            },
            Some(WorkdayOverride {
                kind: WorkdayOverrideKind::UseSchedule,
                work_schedule_id: None,
                ..
            }) => return Err(invalid_data("use_schedule override has no schedule")),
            _ => {
                let selected = self.select_assignment(&command).await?;
                match day_override.as_ref() {
                    Some(value) => SelectedSchedule {
                        work_schedule_id: selected.work_schedule_id,
                        source: WorkScheduleSource::Override,
                        source_id: value.id.clone(),
                    },
                    None => selected,
                }
            }
        };
        let version = self
            .repository
            .find_published_version(&assignment.work_schedule_id, command.work_date)
            .await?
            .ok_or(ResolveWorkdayError::WorkScheduleNotConfigured)?;
        let weekday = u8::try_from(command.work_date.weekday().number_from_monday())
            .map_err(|_| invalid_data("weekday is outside the supported range"))?;
        let rule = self
            .repository
            .find_day_rule(&version.id, weekday)
            .await?
            .ok_or_else(|| invalid_data("published version has no rule for the work date"))?;
        let day_kind = self
            .resolve_day_kind(command.work_date, day_override.as_ref(), &version, &rule)
            .await?;
        let is_working_day = day_kind == ResolvedDayKind::ScheduledWorkday;
        let projection = NewResolvedWorkday {
            user_id: command.user_id,
            work_date: command.work_date,
            work_schedule_id: version.work_schedule_id,
            work_schedule_version_id: version.id,
            source: assignment.source,
            source_id: assignment.source_id,
            day_kind,
            timezone: version.timezone,
            workday_boundary: version.workday_boundary,
            expected_work_minutes: if is_working_day {
                rule.expected_work_minutes
            } else {
                0
            },
            work_intervals: if is_working_day {
                rule.work_intervals
            } else {
                Vec::new()
            },
            planned_breaks: if is_working_day {
                rule.planned_breaks
            } else {
                Vec::new()
            },
            resolved_at: command.resolved_at,
        };
        self.repository.save_projection(projection).await
    }

    async fn select_assignment(
        &self,
        command: &ResolveWorkdayCommand,
    ) -> Result<SelectedSchedule, ResolveWorkdayError> {
        let user_target = AssignmentTarget::User(command.user_id.clone());
        if let Some(assignment) = self
            .repository
            .find_assignment(&user_target, command.work_date)
            .await?
        {
            return Ok(selected_assignment(assignment, WorkScheduleSource::User));
        }

        for department_id in self
            .organization_hierarchy
            .department_lineage(&command.user_id)
            .await?
        {
            let target = AssignmentTarget::Department(department_id);
            if let Some(assignment) = self
                .repository
                .find_assignment(&target, command.work_date)
                .await?
            {
                return Ok(selected_assignment(
                    assignment,
                    WorkScheduleSource::Department,
                ));
            }
        }

        self.repository
            .find_assignment(&AssignmentTarget::Organization, command.work_date)
            .await?
            .map(|assignment| selected_assignment(assignment, WorkScheduleSource::Organization))
            .ok_or(ResolveWorkdayError::WorkScheduleNotConfigured)
    }

    async fn resolve_day_kind(
        &self,
        work_date: NaiveDate,
        day_override: Option<&WorkdayOverride>,
        version: &ScheduleVersion,
        rule: &ScheduleDayRule,
    ) -> Result<ResolvedDayKind, ResolveWorkdayError> {
        match day_override.map(|value| value.kind) {
            Some(WorkdayOverrideKind::NonWorkingDay) => Ok(ResolvedDayKind::ScheduledNonWorkingDay),
            Some(WorkdayOverrideKind::UseSchedule) => Ok(rule_day_kind(rule.day_kind)),
            None => {
                if version.public_holiday_policy == PublicHolidayPolicy::NonWorking
                    && self.holiday_calendar.is_public_holiday(work_date).await?
                {
                    Ok(ResolvedDayKind::PublicHoliday)
                } else {
                    Ok(rule_day_kind(rule.day_kind))
                }
            }
        }
    }
}

#[derive(Debug)]
struct SelectedSchedule {
    work_schedule_id: String,
    source: WorkScheduleSource,
    source_id: String,
}

fn selected_assignment(
    assignment: ScheduleAssignment,
    source: WorkScheduleSource,
) -> SelectedSchedule {
    SelectedSchedule {
        work_schedule_id: assignment.work_schedule_id,
        source,
        source_id: assignment.id,
    }
}

fn rule_day_kind(day_kind: DayKind) -> ResolvedDayKind {
    match day_kind {
        DayKind::WorkingDay => ResolvedDayKind::ScheduledWorkday,
        DayKind::NonWorkingDay => ResolvedDayKind::ScheduledNonWorkingDay,
    }
}

fn invalid_data(message: &str) -> ResolveWorkdayError {
    ResolveWorkdayError::InvalidScheduleData(message.to_string())
}
