use std::rc::Rc;

use timekeeper_contract::admin_attendance_report::{
    AdminAttendanceReportQuery, AdminAttendanceReportResponse,
};

use crate::api::{ApiClient, ApiError};

#[derive(Clone)]
pub struct AttendanceReportRepository {
    client: Rc<ApiClient>,
}

impl AttendanceReportRepository {
    pub fn new(client: Rc<ApiClient>) -> Self {
        Self { client }
    }

    pub async fn fetch(
        &self,
        query: &AdminAttendanceReportQuery,
    ) -> Result<AdminAttendanceReportResponse, ApiError> {
        self.client.admin_attendance_report(query).await
    }
}
