use crate::api::{ApiClient, MfaSetupResponse, MfaStatusResponse};

pub async fn fetch_status() -> Result<MfaStatusResponse, String> {
    ApiClient::new().get_mfa_status().await
}

pub async fn register() -> Result<MfaSetupResponse, String> {
    ApiClient::new().register_mfa().await
}

pub async fn activate(code: &str) -> Result<(), String> {
    ApiClient::new().activate_mfa(code).await
}
