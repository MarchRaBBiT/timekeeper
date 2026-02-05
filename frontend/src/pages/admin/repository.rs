use crate::api::{
    AdminAttendanceUpsert, AdminHolidayKind, AdminHolidayListItem, ApiClient, ApiError,
    CreateHolidayRequest, CreateWeeklyHolidayRequest, HolidayResponse, SubjectRequestListResponse,
    UserResponse, WeeklyHolidayResponse,
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

    pub async fn list_weekly_holidays(&self) -> Result<Vec<WeeklyHolidayResponse>, ApiError> {
        self.client.admin_list_weekly_holidays().await
    }

    pub async fn create_weekly_holiday(
        &self,
        payload: CreateWeeklyHolidayRequest,
    ) -> Result<WeeklyHolidayResponse, ApiError> {
        self.client.admin_create_weekly_holiday(&payload).await
    }

    pub async fn delete_weekly_holiday(&self, id: &str) -> Result<(), ApiError> {
        self.client.admin_delete_weekly_holiday(id).await
    }

    pub async fn list_requests(
        &self,
        status: Option<String>,
        user_id: Option<String>,
        page: u32,
        per_page: u32,
    ) -> Result<Value, ApiError> {
        self.client
            .admin_list_requests(
                status.as_deref(),
                user_id.as_deref(),
                Some(page),
                Some(per_page),
            )
            .await
    }

    pub async fn approve_request(&self, id: &str, comment: &str) -> Result<(), ApiError> {
        self.client
            .admin_approve_request(id, comment)
            .await
            .map(|_| ())
    }

    pub async fn reject_request(&self, id: &str, comment: &str) -> Result<(), ApiError> {
        self.client
            .admin_reject_request(id, comment)
            .await
            .map(|_| ())
    }

    pub async fn list_subject_requests(
        &self,
        status: Option<String>,
        request_type: Option<String>,
        user_id: Option<String>,
        page: i64,
        per_page: i64,
    ) -> Result<SubjectRequestListResponse, ApiError> {
        self.client
            .admin_list_subject_requests(status, request_type, user_id, None, None, page, per_page)
            .await
    }

    pub async fn approve_subject_request(&self, id: &str, comment: &str) -> Result<(), ApiError> {
        self.client.admin_approve_subject_request(id, comment).await
    }

    pub async fn reject_subject_request(&self, id: &str, comment: &str) -> Result<(), ApiError> {
        self.client.admin_reject_subject_request(id, comment).await
    }

    pub async fn reset_mfa(&self, user_id: &str) -> Result<(), ApiError> {
        self.client.admin_reset_mfa(user_id).await.map(|_| ())
    }

    pub async fn upsert_attendance(&self, payload: AdminAttendanceUpsert) -> Result<(), ApiError> {
        self.client
            .admin_upsert_attendance(payload)
            .await
            .map(|_| ())
    }

    pub async fn force_end_break(&self, break_id: &str) -> Result<(), ApiError> {
        self.client
            .admin_force_end_break(break_id)
            .await
            .map(|_| ())
    }

    pub async fn list_holidays(
        &self,
        query: HolidayListQuery,
    ) -> Result<HolidayListResult, ApiError> {
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
    ) -> Result<Vec<CreateHolidayRequest>, ApiError> {
        self.client.admin_fetch_google_holidays(year).await
    }

    pub async fn create_holiday(
        &self,
        payload: CreateHolidayRequest,
    ) -> Result<HolidayResponse, ApiError> {
        self.client.admin_create_holiday(&payload).await
    }

    pub async fn delete_holiday(&self, id: &str) -> Result<(), ApiError> {
        self.client.admin_delete_holiday(id).await.map(|_| ())
    }

    pub async fn fetch_users(&self) -> Result<Vec<UserResponse>, ApiError> {
        self.client.get_users().await
    }
}

#[cfg(all(test, not(target_arch = "wasm32"), not(coverage)))]
mod host_tests {
    use super::*;
    use httpmock::prelude::*;

    fn repo(server: &MockServer) -> AdminRepository {
        AdminRepository::new_with_client(std::rc::Rc::new(ApiClient::new_with_base_url(
            &server.url("/api"),
        )))
    }

    fn weekly_holiday_json() -> serde_json::Value {
        serde_json::json!({
            "id": "wh1",
            "weekday": 1,
            "starts_on": "2025-01-01",
            "ends_on": null,
            "enforced_from": "2025-01-01",
            "enforced_to": null
        })
    }

    #[tokio::test]
    async fn admin_repository_calls_endpoints() {
        let server = MockServer::start_async().await;
        server.mock(|when, then| {
            when.method(GET).path("/api/admin/holidays/weekly");
            then.status(200).json_body(serde_json::json!([weekly_holiday_json()]));
        });
        server.mock(|when, then| {
            when.method(POST).path("/api/admin/holidays/weekly");
            then.status(200).json_body(weekly_holiday_json());
        });
        server.mock(|when, then| {
            when.method(DELETE).path("/api/admin/holidays/weekly/wh1");
            then.status(200).json_body(serde_json::json!({}));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/admin/requests");
            then.status(200).json_body(serde_json::json!({ "items": [] }));
        });
        server.mock(|when, then| {
            when.method(PUT).path("/api/admin/requests/req-1/approve");
            then.status(200).json_body(serde_json::json!({ "status": "approved" }));
        });
        server.mock(|when, then| {
            when.method(PUT).path("/api/admin/requests/req-1/reject");
            then.status(200).json_body(serde_json::json!({ "status": "rejected" }));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/admin/subject-requests");
            then.status(200).json_body(serde_json::json!({
                "page": 1,
                "per_page": 20,
                "total": 0,
                "items": []
            }));
        });
        server.mock(|when, then| {
            when.method(PUT).path("/api/admin/subject-requests/sr-1/approve");
            then.status(200).json_body(serde_json::json!({}));
        });
        server.mock(|when, then| {
            when.method(PUT).path("/api/admin/subject-requests/sr-1/reject");
            then.status(200).json_body(serde_json::json!({}));
        });
        server.mock(|when, then| {
            when.method(POST).path("/api/admin/mfa/reset");
            then.status(200).json_body(serde_json::json!({}));
        });
        server.mock(|when, then| {
            when.method(PUT).path("/api/admin/attendance");
            then.status(200).json_body(serde_json::json!({
                "id": "att-1",
                "user_id": "u1",
                "date": "2025-01-02",
                "clock_in_time": "2025-01-02T09:00:00",
                "clock_out_time": null,
                "status": "clocked_in",
                "total_work_hours": null,
                "break_records": []
            }));
        });
        server.mock(|when, then| {
            when.method(PUT).path("/api/admin/breaks/br-1/force-end");
            then.status(200).json_body(serde_json::json!({
                "id": "br-1",
                "attendance_id": "att-1",
                "break_start_time": "2025-01-02T12:00:00",
                "break_end_time": null,
                "duration_minutes": null
            }));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/admin/holidays");
            then.status(200).json_body(serde_json::json!({
                "page": 1,
                "per_page": 50,
                "total": 0,
                "items": []
            }));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/admin/holidays/google");
            then.status(200).json_body(serde_json::json!([{
                "holiday_date": "2025-01-03",
                "name": "Imported",
                "description": null
            }]));
        });
        server.mock(|when, then| {
            when.method(POST).path("/api/admin/holidays");
            then.status(200).json_body(serde_json::json!({
                "id": "h1",
                "holiday_date": "2025-01-03",
                "name": "Imported",
                "description": null
            }));
        });
        server.mock(|when, then| {
            when.method(DELETE).path("/api/admin/holidays/h1");
            then.status(200).json_body(serde_json::json!({}));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/admin/users");
            then.status(200).json_body(serde_json::json!([{
                "id": "u1",
                "username": "alice",
                "full_name": "Alice Example",
                "role": "admin",
                "is_system_admin": true,
                "mfa_enabled": false
            }]));
        });

        let repo = repo(&server);
        assert_eq!(repo.list_weekly_holidays().await.unwrap().len(), 1);
        repo.create_weekly_holiday(CreateWeeklyHolidayRequest {
            weekday: 1,
            starts_on: chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            ends_on: None,
        }).await.unwrap();
        repo.delete_weekly_holiday("wh1").await.unwrap();
        repo.list_requests(None, None, 1, 20).await.unwrap();
        repo.approve_request("req-1", "ok").await.unwrap();
        repo.reject_request("req-1", "no").await.unwrap();
        repo.list_subject_requests(None, None, None, 1, 20).await.unwrap();
        repo.approve_subject_request("sr-1", "ok").await.unwrap();
        repo.reject_subject_request("sr-1", "no").await.unwrap();
        repo.reset_mfa("u1").await.unwrap();
        repo.upsert_attendance(AdminAttendanceUpsert {
            user_id: "u1".into(),
            date: chrono::NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
            clock_in_time: chrono::NaiveDate::from_ymd_opt(2025, 1, 2)
                .unwrap()
                .and_hms_opt(9, 0, 0)
                .unwrap(),
            clock_out_time: None,
            breaks: None,
        }).await.unwrap();
        repo.force_end_break("br-1").await.unwrap();
        repo.list_holidays(HolidayListQuery::default()).await.unwrap();
        repo.fetch_google_holidays(Some(2025)).await.unwrap();
        repo.create_holiday(CreateHolidayRequest {
            holiday_date: chrono::NaiveDate::from_ymd_opt(2025, 1, 3).unwrap(),
            name: "Imported".into(),
            description: None,
        }).await.unwrap();
        repo.delete_holiday("h1").await.unwrap();
        assert_eq!(repo.fetch_users().await.unwrap().len(), 1);
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
