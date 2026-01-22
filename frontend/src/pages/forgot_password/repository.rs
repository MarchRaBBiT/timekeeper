use crate::api::{ApiClient, ApiError, MessageResponse};
use std::rc::Rc;

#[derive(Clone)]
pub struct ForgotPasswordRepository {
    client: Rc<ApiClient>,
}

impl ForgotPasswordRepository {
    pub fn new_with_client(client: Rc<ApiClient>) -> Self {
        Self { client }
    }

    pub async fn request_reset(&self, email: String) -> Result<MessageResponse, ApiError> {
        self.client.request_password_reset(email).await
    }
}
