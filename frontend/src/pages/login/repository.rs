use crate::api::{ApiClient, LoginRequest, LoginResponse};

pub async fn login(request: LoginRequest) -> Result<LoginResponse, String> {
    ApiClient::new().login(request).await
}

pub async fn logout(all: bool) -> Result<(), String> {
    ApiClient::new().logout(all).await
}
