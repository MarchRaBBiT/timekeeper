use crate::api::{
    AdminAttendanceUpsert, ApiClient, CreateHolidayRequest, CreateWeeklyHolidayRequest,
    HolidayResponse, WeeklyHolidayResponse,
};
use serde_json::Value;
use std::rc::Rc;

#[derive(Clone)]
pub struct AdminRepository {
    client: Rc<ApiClient>,
}

impl Default for AdminRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl AdminRepository {
    pub fn new() -> Self {
        Self {
            client: Rc::new(ApiClient::new()),
        }
    }

    pub fn with_client(client: ApiClient) -> Self {
        Self {
            client: Rc::new(client),
        }
    }

    pub async fn list_weekly_holidays(&self) -> Result<Vec<WeeklyHolidayResponse>, String> {
        self.client.admin_list_weekly_holidays().await
    }

    pub async fn create_weekly_holiday(
        &self,
        payload: CreateWeeklyHolidayRequest,
    ) -> Result<WeeklyHolidayResponse, String> {
        self.client.admin_create_weekly_holiday(&payload).await
    }

    pub async fn list_requests(
        &self,
        status: Option<String>,
        user_id: Option<String>,
        page: u32,
        per_page: u32,
    ) -> Result<Value, String> {
        self.client
            .admin_list_requests(
                status.as_deref(),
                user_id.as_deref(),
                Some(page),
                Some(per_page),
            )
            .await
    }

    pub async fn approve_request(&self, id: &str, comment: &str) -> Result<(), String> {
        self.client
            .admin_approve_request(id, comment)
            .await
            .map(|_| ())
    }

    pub async fn reject_request(&self, id: &str, comment: &str) -> Result<(), String> {
        self.client
            .admin_reject_request(id, comment)
            .await
            .map(|_| ())
    }

    pub async fn reset_mfa(&self, user_id: &str) -> Result<(), String> {
        self.client.admin_reset_mfa(user_id).await.map(|_| ())
    }

    pub async fn upsert_attendance(&self, payload: AdminAttendanceUpsert) -> Result<(), String> {
        self.client
            .admin_upsert_attendance(payload)
            .await
            .map(|_| ())
    }

    pub async fn force_end_break(&self, break_id: &str) -> Result<(), String> {
        self.client
            .admin_force_end_break(break_id)
            .await
            .map(|_| ())
    }

    pub async fn list_holidays(&self) -> Result<Vec<HolidayResponse>, String> {
        self.client.admin_list_holidays().await
    }

    pub async fn fetch_google_holidays(
        &self,
        year: Option<i32>,
    ) -> Result<Vec<CreateHolidayRequest>, String> {
        self.client.admin_fetch_google_holidays(year).await
    }

    pub async fn create_holiday(
        &self,
        payload: CreateHolidayRequest,
    ) -> Result<HolidayResponse, String> {
        self.client.admin_create_holiday(&payload).await
    }

    pub async fn delete_holiday(&self, id: &str) -> Result<(), String> {
        self.client.admin_delete_holiday(id).await.map(|_| ())
    }
}
