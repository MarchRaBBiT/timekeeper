use crate::api::{ApiClient, ApiError, LoginRequest, LoginResponse};
use std::rc::Rc;

#[derive(Clone)]
pub struct LoginRepository {
    client: Rc<ApiClient>,
}

impl LoginRepository {
    pub fn new_with_client(client: Rc<ApiClient>) -> Self {
        Self { client }
    }

    pub async fn login(&self, request: LoginRequest) -> Result<LoginResponse, ApiError> {
        self.client.login(request).await
    }

    pub async fn logout(&self, all: bool) -> Result<(), ApiError> {
        self.client.logout(all).await
    }
}
