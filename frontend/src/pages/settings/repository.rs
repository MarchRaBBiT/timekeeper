use crate::api::{ApiClient, ApiError};

pub async fn change_password(
    client: ApiClient,
    current: String,
    new: String,
) -> Result<(), ApiError> {
    client.change_password(current, new).await
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::change_password;
    use crate::api::test_support::mock::*;
    use crate::api::ApiClient;

    #[tokio::test]
    async fn change_password_calls_api() {
        let server = MockServer::start_async().await;
        server.mock(|when, then| {
            when.method(PUT).path("/api/auth/change-password");
            then.status(200).json_body(serde_json::json!({}));
        });

        let client = ApiClient::new_with_base_url(&server.url("/api"));
        change_password(client, "current".into(), "newpass".into())
            .await
            .expect("change password");
    }
}
