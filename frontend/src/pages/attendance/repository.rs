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

#[cfg(all(test, not(target_arch = "wasm32"), not(coverage)))]
mod host_tests {
    use super::*;
    use httpmock::prelude::*;

    #[tokio::test]
    async fn attendance_repository_calls_api() {
        let server = MockServer::start_async().await;
        server.mock(|when, then| {
            when.method(GET).path("/api/holidays/month");
            then.status(200).json_body(serde_json::json!([{
                "date": "2025-01-01",
                "reason": "public holiday"
            }]));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/attendance/att-1/breaks");
            then.status(200).json_body(serde_json::json!([{
                "id": "br-1",
                "attendance_id": "att-1",
                "break_start_time": "2025-01-02T12:00:00",
                "break_end_time": null,
                "duration_minutes": null
            }]));
        });

        let api = ApiClient::new_with_base_url(&server.url("/api"));
        let holidays = fetch_monthly_holidays(&api, 2025, 1).await.unwrap();
        assert_eq!(holidays.len(), 1);
        let breaks = fetch_breaks_by_attendance(&api, "att-1").await.unwrap();
        assert_eq!(breaks.len(), 1);
    }
}
