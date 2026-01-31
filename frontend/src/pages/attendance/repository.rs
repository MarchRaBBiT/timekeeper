use crate::api::{ApiClient, ApiError, BreakRecordResponse, HolidayCalendarEntry};

pub async fn fetch_monthly_holidays(
    api: &ApiClient,
    year: i32,
    month: u32,
) -> Result<Vec<HolidayCalendarEntry>, ApiError> {
    api.get_monthly_holidays(year, month).await
}

pub async fn fetch_breaks_by_attendance(
    api: &ApiClient,
    attendance_id: &str,
) -> Result<Vec<BreakRecordResponse>, ApiError> {
    api.get_breaks_by_attendance(attendance_id).await
}
