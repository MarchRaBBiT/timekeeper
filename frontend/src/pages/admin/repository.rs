use crate::api::{
    AdminAttendanceUpsert, ApiClient, CreateHolidayRequest, CreateWeeklyHolidayRequest,
    HolidayResponse, WeeklyHolidayResponse,
};
use serde_json::Value;

pub async fn list_weekly_holidays() -> Result<Vec<WeeklyHolidayResponse>, String> {
    ApiClient::new().admin_list_weekly_holidays().await
}

pub async fn create_weekly_holiday(
    payload: CreateWeeklyHolidayRequest,
) -> Result<WeeklyHolidayResponse, String> {
    ApiClient::new().admin_create_weekly_holiday(&payload).await
}

pub async fn list_requests(
    status: Option<String>,
    user_id: Option<String>,
    page: u32,
    per_page: u32,
) -> Result<Value, String> {
    ApiClient::new()
        .admin_list_requests(
            status.as_deref(),
            user_id.as_deref(),
            Some(page),
            Some(per_page),
        )
        .await
}

pub async fn approve_request(id: &str, comment: &str) -> Result<(), String> {
    ApiClient::new()
        .admin_approve_request(id, comment)
        .await
        .map(|_| ())
}

pub async fn reject_request(id: &str, comment: &str) -> Result<(), String> {
    ApiClient::new()
        .admin_reject_request(id, comment)
        .await
        .map(|_| ())
}

pub async fn reset_mfa(user_id: &str) -> Result<(), String> {
    ApiClient::new().admin_reset_mfa(user_id).await.map(|_| ())
}

pub async fn upsert_attendance(payload: AdminAttendanceUpsert) -> Result<(), String> {
    ApiClient::new()
        .admin_upsert_attendance(payload)
        .await
        .map(|_| ())
}

pub async fn force_end_break(break_id: &str) -> Result<(), String> {
    ApiClient::new()
        .admin_force_end_break(break_id)
        .await
        .map(|_| ())
}

pub async fn list_holidays() -> Result<Vec<HolidayResponse>, String> {
    ApiClient::new().admin_list_holidays().await
}

pub async fn fetch_google_holidays(year: Option<i32>) -> Result<Vec<CreateHolidayRequest>, String> {
    ApiClient::new().admin_fetch_google_holidays(year).await
}

pub async fn create_holiday(payload: CreateHolidayRequest) -> Result<HolidayResponse, String> {
    ApiClient::new().admin_create_holiday(&payload).await
}

pub async fn delete_holiday(id: &str) -> Result<(), String> {
    ApiClient::new().admin_delete_holiday(id).await.map(|_| ())
}
