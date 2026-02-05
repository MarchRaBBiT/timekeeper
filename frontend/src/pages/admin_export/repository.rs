use crate::api::{ApiClient, ApiError};
use std::rc::Rc;

#[derive(Clone)]
pub struct AdminExportRepository {
    client: Rc<ApiClient>,
}

impl Default for AdminExportRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl AdminExportRepository {
    pub fn new() -> Self {
        Self {
            client: Rc::new(ApiClient::new()),
        }
    }

    pub fn new_with_client(client: Rc<ApiClient>) -> Self {
        Self { client }
    }

    pub async fn export_data_filtered(
        &self,
        username: Option<&str>,
        from: Option<&str>,
        to: Option<&str>,
    ) -> Result<serde_json::Value, ApiError> {
        self.client.export_data_filtered(username, from, to).await
    }

    pub async fn fetch_users(&self) -> Result<Vec<crate::api::UserResponse>, ApiError> {
        self.client.get_users().await
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::api::test_support::mock::*;

    fn repository(server: &MockServer) -> AdminExportRepository {
        AdminExportRepository::new_with_client(Rc::new(ApiClient::new_with_base_url(
            &server.url("/api"),
        )))
    }

    #[tokio::test]
    async fn admin_export_repository_calls_endpoints() {
        let server = MockServer::start_async().await;
        server.mock(|when, then| {
            when.method(GET).path("/api/admin/export");
            then.status(200)
                .json_body(serde_json::json!({ "filename": "export.csv", "csv_data": "a,b\n1,2" }));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/admin/users");
            then.status(200).json_body(serde_json::json!([{
                "id": "u1",
                "username": "alice",
                "full_name": "Alice",
                "role": "member",
                "is_system_admin": false,
                "mfa_enabled": false
            }]));
        });

        let repo = repository(&server);
        let export = repo
            .export_data_filtered(Some("alice"), Some("2026-01-01"), Some("2026-01-31"))
            .await
            .unwrap();
        assert_eq!(export["filename"], "export.csv");

        let users = repo.fetch_users().await.unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].username, "alice");
    }

    #[tokio::test]
    async fn export_data_filtered_propagates_request_error() {
        let server = MockServer::start_async().await;
        server.mock(|when, then| {
            when.method(GET).path("/api/admin/export");
            then.status(500).json_body(serde_json::json!({
                "error": "failed to generate export",
                "code": "INTERNAL_SERVER_ERROR"
            }));
        });

        let repo = repository(&server);
        let error = repo
            .export_data_filtered(None, None, None)
            .await
            .expect_err("should return API error");
        assert_eq!(error.code, "INTERNAL_SERVER_ERROR");
    }
}
