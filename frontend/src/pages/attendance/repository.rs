use crate::api::{ApiClient, BreakRecordResponse, HolidayCalendarEntry};
use serde_json::Value;

pub async fn fetch_monthly_holidays(
    api: &ApiClient,
    year: i32,
    month: u32,
) -> Result<Vec<HolidayCalendarEntry>, String> {
    api.get_monthly_holidays(year, month).await
}

pub async fn export_attendance_csv(
    api: &ApiClient,
    from: Option<&str>,
    to: Option<&str>,
) -> Result<Value, String> {
    api.export_my_attendance_filtered(from, to).await
}

pub async fn fetch_breaks_by_attendance(
    api: &ApiClient,
    attendance_id: &str,
) -> Result<Vec<BreakRecordResponse>, String> {
    api.get_breaks_by_attendance(attendance_id).await
}
