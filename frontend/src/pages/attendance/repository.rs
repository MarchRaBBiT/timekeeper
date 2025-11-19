use crate::api::{ApiClient, BreakRecordResponse, HolidayCalendarEntry};
use serde_json::Value;

pub async fn fetch_monthly_holidays(
    year: i32,
    month: u32,
) -> Result<Vec<HolidayCalendarEntry>, String> {
    ApiClient::new().get_monthly_holidays(year, month).await
}

pub async fn export_attendance_csv(from: Option<&str>, to: Option<&str>) -> Result<Value, String> {
    ApiClient::new()
        .export_my_attendance_filtered(from, to)
        .await
}

pub async fn fetch_breaks_by_attendance(
    attendance_id: &str,
) -> Result<Vec<BreakRecordResponse>, String> {
    ApiClient::new()
        .get_breaks_by_attendance(attendance_id)
        .await
}
