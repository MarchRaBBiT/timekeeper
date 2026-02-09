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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::api::test_support::mock::*;

    fn repository(server: &MockServer) -> ResetPasswordRepository {
        ResetPasswordRepository::new_with_client(Rc::new(ApiClient::new_with_base_url(
            &server.url("/api"),
        )))
    }

    #[tokio::test]
    async fn reset_password_calls_api() {
        let server = MockServer::start_async().await;
        server.mock(|when, then| {
            when.method(POST).path("/api/auth/reset-password");
            then.status(200)
                .json_body(serde_json::json!({ "message": "password reset complete" }));
        });

        let repo = repository(&server);
        let response = repo
            .reset_password("valid-token".into(), "new-password".into())
            .await
            .unwrap();
        assert_eq!(response.message, "password reset complete");
    }

    #[tokio::test]
    async fn reset_password_propagates_validation_error() {
        let server = MockServer::start_async().await;
        server.mock(|when, then| {
            when.method(POST).path("/api/auth/reset-password");
            then.status(400).json_body(serde_json::json!({
                "error": "invalid token",
                "code": "VALIDATION_ERROR"
            }));
        });

        let repo = repository(&server);
        let error = repo
            .reset_password("expired-token".into(), "new-password".into())
            .await
            .expect_err("should return validation error");
        assert_eq!(error.code, "VALIDATION_ERROR");
    }
}
