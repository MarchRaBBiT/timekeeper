use crate::api::{ApiClient, ApiError, SessionResponse};

pub async fn change_password(
    client: ApiClient,
    current: String,
    new: String,
) -> Result<(), ApiError> {
    client.change_password(current, new).await
}

pub async fn list_sessions(client: ApiClient) -> Result<Vec<SessionResponse>, ApiError> {
    client.list_sessions().await
}

pub async fn revoke_session(client: ApiClient, session_id: String) -> Result<(), ApiError> {
    client.revoke_session(&session_id).await
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::{change_password, list_sessions, revoke_session};
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

    #[tokio::test]
    async fn session_repository_calls_api() {
        let server = MockServer::start_async().await;
        server.mock(|when, then| {
            when.method(GET).path("/api/auth/sessions");
            then.status(200).json_body(serde_json::json!([{
                "id": "session-1",
                "device_label": "Chrome on macOS",
                "created_at": "2026-03-10T10:00:00Z",
                "last_seen_at": "2026-03-10T10:30:00Z",
                "expires_at": "2026-03-17T10:00:00Z",
                "is_current": true
            }]));
        });
        server.mock(|when, then| {
            when.method(DELETE).path("/api/auth/sessions/session-1");
            then.status(200).json_body(serde_json::json!({}));
        });

        let client = ApiClient::new_with_base_url(&server.url("/api"));
        let sessions = list_sessions(client.clone()).await.expect("list sessions");
        assert_eq!(sessions.len(), 1);
        revoke_session(client, "session-1".into())
            .await
            .expect("revoke session");
    }
}
