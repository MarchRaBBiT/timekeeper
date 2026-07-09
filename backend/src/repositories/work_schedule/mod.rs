mod assignments;
mod master;
mod operations;
mod rows;
mod versions;

pub use assignments::{
    create_assignment, delete_assignment, list_assignments, AssignmentListFilter,
};
pub use master::{
    create_work_schedule, find_work_schedule, get_work_schedule_detail, list_work_schedules,
    retire_work_schedule, update_work_schedule, WorkScheduleListFilter,
};
pub use operations::{
    close_month, get_overtime_monitor_settings, list_anomalies, list_overtime_monitor,
    list_user_attendance_calendar, list_user_leave_calendar, transition_monthly_closing,
    upsert_overtime_monitor_settings,
};
pub use versions::{
    create_version, delete_version, find_version, publish_version, replace_version,
};

#[derive(Debug, thiserror::Error)]
pub enum WorkScheduleRepositoryError {
    #[error("work schedule resource was not found")]
    NotFound,
    #[error("work schedule code already exists")]
    CodeConflict,
    #[error("effective period overlaps an existing record")]
    PeriodOverlap,
    #[error("published work schedule version is immutable")]
    PublishedVersionImmutable,
    #[error("draft revision does not match")]
    RevisionConflict,
    #[error("retired work schedule cannot be assigned")]
    RetiredSchedule,
    #[error("referenced user or department does not exist")]
    InvalidReference,
    #[error("invalid monthly closing state transition")]
    InvalidStateTransition,
    #[error("stored work schedule data is invalid: {0}")]
    CorruptData(String),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

pub type RepositoryResult<T> = Result<T, WorkScheduleRepositoryError>;

fn map_database_error(error: sqlx::Error) -> WorkScheduleRepositoryError {
    let sqlx::Error::Database(database) = &error else {
        return WorkScheduleRepositoryError::Sqlx(error);
    };
    match database.constraint() {
        Some("work_schedules_code_ci_key") => WorkScheduleRepositoryError::CodeConflict,
        Some("work_schedule_versions_published_period_excl")
        | Some("work_schedule_assignments_target_period_excl") => {
            WorkScheduleRepositoryError::PeriodOverlap
        }
        Some(constraint) if constraint.ends_with("_fkey") => {
            WorkScheduleRepositoryError::InvalidReference
        }
        _ => WorkScheduleRepositoryError::Sqlx(error),
    }
}
