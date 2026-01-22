use crate::api::{ApiClient, ApiError, MessageResponse};
use std::rc::Rc;

#[derive(Clone)]
pub struct ResetPasswordRepository {
    client: Rc<ApiClient>,
}

impl ResetPasswordRepository {
    pub fn new_with_client(client: Rc<ApiClient>) -> Self {
        Self { client }
    }

    pub async fn reset_password(
        &self,
        token: String,
        new_password: String,
    ) -> Result<MessageResponse, ApiError> {
        self.client.reset_password(token, new_password).await
    }
}
