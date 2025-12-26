use crate::api::{
    AdminAttendanceUpsert, AdminHolidayKind, AdminHolidayListItem, ApiClient, CreateHolidayRequest,
    CreateWeeklyHolidayRequest, HolidayResponse, UserResponse, WeeklyHolidayResponse,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HolidayListQuery {
    pub page: i64,
    pub per_page: i64,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

impl Default for HolidayListQuery {
    fn default() -> Self {
        Self {
            page: 1,
            per_page: 10,
            from: None,
            to: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HolidayListResult {
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub items: Vec<HolidayResponse>,
}

impl HolidayListResult {
    pub fn empty(page: i64, per_page: i64) -> Self {
        Self {
            page,
            per_page,
            total: 0,
            items: Vec::new(),
        }
    }
}

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

    pub fn new_with_client(client: Rc<ApiClient>) -> Self {
        Self { client }
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

    pub async fn list_holidays(
        &self,
        query: HolidayListQuery,
    ) -> Result<HolidayListResult, String> {
        let response = self
            .client
            .admin_list_holidays(query.page, query.per_page, query.from, query.to)
            .await?;

        let items = response
            .items
            .into_iter()
            .filter_map(convert_admin_holiday_item)
            .collect::<Vec<_>>();

        Ok(HolidayListResult {
            page: response.page,
            per_page: response.per_page,
            total: response.total,
            items,
        })
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

    pub async fn fetch_users(&self) -> Result<Vec<UserResponse>, String> {
        self.client.get_users().await
    }
}

fn convert_admin_holiday_item(item: AdminHolidayListItem) -> Option<HolidayResponse> {
    if item.kind != AdminHolidayKind::Public {
        return None;
    }

    let AdminHolidayListItem {
        id,
        applies_from,
        date,
        name,
        description,
        reason,
        ..
    } = item;

    let fallback_reason = reason.clone();
    let holiday_name = name
        .or_else(|| fallback_reason.clone())
        .unwrap_or_else(|| "Holiday".to_string());
    let holiday_description = description.or(reason);

    Some(HolidayResponse {
        id,
        holiday_date: date.unwrap_or(applies_from),
        name: holiday_name,
        description: holiday_description,
    })
}
