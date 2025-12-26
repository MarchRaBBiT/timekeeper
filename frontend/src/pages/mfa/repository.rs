use crate::api::{ApiClient, MfaSetupResponse, MfaStatusResponse};
use std::rc::Rc;

#[derive(Clone)]
pub struct MfaRepository {
    client: Rc<ApiClient>,
}

impl MfaRepository {
    pub fn new() -> Self {
        Self {
            client: Rc::new(ApiClient::new()),
        }
    }

    pub fn new_with_client(client: Rc<ApiClient>) -> Self {
        Self { client }
    }

    pub async fn fetch_status(&self) -> Result<MfaStatusResponse, String> {
        self.client.get_mfa_status().await
    }

    pub async fn register(&self) -> Result<MfaSetupResponse, String> {
        self.client.register_mfa().await
    }

    pub async fn activate(&self, code: &str) -> Result<(), String> {
        self.client.activate_mfa(code).await
    }
}
