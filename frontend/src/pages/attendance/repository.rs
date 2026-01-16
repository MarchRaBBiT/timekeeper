use crate::api::{ApiClient, ApiError, BreakRecordResponse, HolidayCalendarEntry};
use serde_json::Value;

pub async fn fetch_monthly_holidays(
    api: &ApiClient,
    year: i32,
    month: u32,
) -> Result<Vec<HolidayCalendarEntry>, ApiError> {
    api.get_monthly_holidays(year, month).await
}

// TODO: リファクタリング後に使用可否を判断
// - 使う可能性: あり
// - 想定機能: 勤怠CSVエクスポート
#[allow(dead_code)]
pub async fn export_attendance_csv(
    api: &ApiClient,
    from: Option<&str>,
    to: Option<&str>,
) -> Result<Value, ApiError> {
    api.export_my_attendance_filtered(from, to).await
}

pub async fn fetch_breaks_by_attendance(
    api: &ApiClient,
    attendance_id: &str,
) -> Result<Vec<BreakRecordResponse>, ApiError> {
    api.get_breaks_by_attendance(attendance_id).await
}
