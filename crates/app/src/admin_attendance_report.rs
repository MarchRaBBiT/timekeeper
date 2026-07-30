use async_trait::async_trait;
use thiserror::Error;
use timekeeper_contract::{
    admin_attendance_report::{
        AdminAttendanceReportItem, AdminAttendanceReportQuery, AdminAttendanceReportResponse,
    },
    work_schedules::OvertimeMonitorStatus,
};

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum AdminAttendanceReportError {
    #[error("year must be between 1900 and 9999")]
    InvalidYear,
    #[error("month must be between 1 and 12")]
    InvalidMonth,
    #[error("page must be between 1 and 1000")]
    InvalidPage,
    #[error("per_page must be between 1 and 100")]
    InvalidPerPage,
    #[error("department is outside the actor scope")]
    ForbiddenDepartment,
    #[error("attendance report repository error: {0}")]
    Repository(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminAttendanceReportCommand {
    pub actor_user_id: String,
    pub actor_is_system_admin: bool,
    pub query: AdminAttendanceReportQuery,
}

#[async_trait]
pub trait AdminAttendanceReportRepository: Send + Sync {
    /// One composed item per scoped user, loaded with month/scope batch reads.
    async fn list_scoped_month(
        &self,
        actor_user_id: &str,
        actor_is_system_admin: bool,
        year: i32,
        month: u32,
        department_id: Option<&str>,
    ) -> Result<Vec<AdminAttendanceReportItem>, AdminAttendanceReportError>;
}

#[derive(Debug, Clone)]
pub struct GetAdminAttendanceReport<R> {
    repository: R,
}

impl<R: AdminAttendanceReportRepository> GetAdminAttendanceReport<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        command: AdminAttendanceReportCommand,
    ) -> Result<AdminAttendanceReportResponse, AdminAttendanceReportError> {
        validate_query(&command.query)?;
        let query = command.query;
        let mut items = self
            .repository
            .list_scoped_month(
                &command.actor_user_id,
                command.actor_is_system_admin,
                query.year,
                query.month,
                query.department_id.as_deref(),
            )
            .await?;
        items.sort_by(|left, right| {
            severity_rank(right.overtime_status)
                .cmp(&severity_rank(left.overtime_status))
                .then_with(|| left.user_id.cmp(&right.user_id))
        });
        let total = i64::try_from(items.len())
            .map_err(|error| AdminAttendanceReportError::Repository(error.to_string()))?;
        let offset = usize::try_from((query.page - 1) * query.per_page)
            .map_err(|error| AdminAttendanceReportError::Repository(error.to_string()))?;
        let page_size = usize::try_from(query.per_page)
            .map_err(|error| AdminAttendanceReportError::Repository(error.to_string()))?;
        Ok(AdminAttendanceReportResponse {
            year: query.year,
            month: query.month,
            page: query.page,
            per_page: query.per_page,
            total,
            items: items.into_iter().skip(offset).take(page_size).collect(),
        })
    }
}

fn validate_query(query: &AdminAttendanceReportQuery) -> Result<(), AdminAttendanceReportError> {
    if !(1900..=9999).contains(&query.year) {
        return Err(AdminAttendanceReportError::InvalidYear);
    }
    if !(1..=12).contains(&query.month) {
        return Err(AdminAttendanceReportError::InvalidMonth);
    }
    if !(1..=1_000).contains(&query.page) {
        return Err(AdminAttendanceReportError::InvalidPage);
    }
    if !(1..=100).contains(&query.per_page) {
        return Err(AdminAttendanceReportError::InvalidPerPage);
    }
    Ok(())
}

const fn severity_rank(status: OvertimeMonitorStatus) -> u8 {
    match status {
        OvertimeMonitorStatus::Ok => 0,
        OvertimeMonitorStatus::Warning => 1,
        OvertimeMonitorStatus::Exceeded => 2,
    }
}
