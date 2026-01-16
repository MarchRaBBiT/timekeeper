use crate::api::{ApiClient, ApiError, MfaSetupResponse, MfaStatusResponse};
use std::rc::Rc;

#[derive(Clone)]
pub struct MfaRepository {
    client: Rc<ApiClient>,
}

impl MfaRepository {
    // TODO: リファクタリング後に使用可否を判断
    // - 使う可能性: あり
    // - 想定機能: MFA画面のRepository生成
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            client: Rc::new(ApiClient::new()),
        }
    }

    pub fn new_with_client(client: Rc<ApiClient>) -> Self {
        Self { client }
    }

    pub async fn fetch_status(&self) -> Result<MfaStatusResponse, ApiError> {
        self.client.get_mfa_status().await
    }

    pub async fn register(&self) -> Result<MfaSetupResponse, ApiError> {
        self.client.register_mfa().await
    }

    pub async fn activate(&self, code: &str) -> Result<(), ApiError> {
        self.client.activate_mfa(code).await
    }
}
