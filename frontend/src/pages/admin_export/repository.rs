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
