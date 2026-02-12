use crate::api::{
    ApiClient, ApiError, ArchivedUserResponse, CreateUser, PiiProtectedResponse, UserResponse,
};
use std::rc::Rc;

#[derive(Clone)]
pub struct AdminUsersRepository {
    client: Rc<ApiClient>,
}

impl Default for AdminUsersRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl AdminUsersRepository {
    pub fn new() -> Self {
        Self {
            client: std::rc::Rc::new(ApiClient::new()),
        }
    }

    pub fn new_with_client(client: Rc<ApiClient>) -> Self {
        Self { client }
    }

    pub async fn fetch_users(&self) -> Result<PiiProtectedResponse<Vec<UserResponse>>, ApiError> {
        self.client.get_users_with_policy().await
    }

    pub async fn invite_user(&self, payload: CreateUser) -> Result<UserResponse, ApiError> {
        self.client.create_user(payload).await
    }

    pub async fn reset_user_mfa(&self, user_id: String) -> Result<(), ApiError> {
        self.client.admin_reset_mfa(&user_id).await
    }

    /// Delete a user (soft delete by default, hard delete if `hard` is true).
    pub async fn delete_user(&self, user_id: String, hard: bool) -> Result<(), ApiError> {
        self.client.admin_delete_user(&user_id, hard).await
    }

    pub async fn unlock_user(&self, user_id: String) -> Result<(), ApiError> {
        self.client.admin_unlock_user(&user_id).await
    }

    // ========================================================================
    // Archived user functions
    // ========================================================================

    pub async fn fetch_archived_users(&self) -> Result<Vec<ArchivedUserResponse>, ApiError> {
        self.client.admin_get_archived_users().await
    }

    pub async fn restore_archived_user(&self, user_id: String) -> Result<(), ApiError> {
        self.client.admin_restore_archived_user(&user_id).await
    }

    pub async fn delete_archived_user(&self, user_id: String) -> Result<(), ApiError> {
        self.client.admin_delete_archived_user(&user_id).await
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::api::test_support::mock::*;

    fn repository(server: &MockServer) -> AdminUsersRepository {
        AdminUsersRepository::new_with_client(Rc::new(ApiClient::new_with_base_url(
            &server.url("/api"),
        )))
    }

    fn user_json(id: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "username": format!("user-{}", id),
            "full_name": format!("User {}", id),
            "role": "member",
            "is_system_admin": false,
            "mfa_enabled": false
        })
    }

    fn archived_user_json(id: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "username": format!("archived-{}", id),
            "full_name": format!("Archived {}", id),
            "role": "member",
            "is_system_admin": false,
            "archived_at": "2026-01-01T00:00:00Z",
            "archived_by": "admin-1"
        })
    }

    #[tokio::test]
    async fn admin_users_repository_calls_endpoints() {
        let server = MockServer::start_async().await;
        server.mock(|when, then| {
            when.method(GET).path("/api/admin/users");
            then.status(200)
                .json_body(serde_json::json!([user_json("u1")]));
        });
        server.mock(|when, then| {
            when.method(POST).path("/api/admin/users");
            then.status(200).json_body(user_json("u2"));
        });
        server.mock(|when, then| {
            when.method(POST).path("/api/admin/mfa/reset");
            then.status(200)
                .json_body(serde_json::json!({ "status": "ok" }));
        });
        server.mock(|when, then| {
            when.method(DELETE).path("/api/admin/users/u1");
            then.status(200)
                .json_body(serde_json::json!({ "status": "deleted" }));
        });
        server.mock(|when, then| {
            when.method(POST).path("/api/admin/users/u1/unlock");
            then.status(200)
                .json_body(serde_json::json!({ "status": "unlocked" }));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/admin/archived-users");
            then.status(200)
                .json_body(serde_json::json!([archived_user_json("a1")]));
        });
        server.mock(|when, then| {
            when.method(POST)
                .path("/api/admin/archived-users/a1/restore");
            then.status(200)
                .json_body(serde_json::json!({ "status": "restored" }));
        });
        server.mock(|when, then| {
            when.method(DELETE).path("/api/admin/archived-users/a1");
            then.status(200)
                .json_body(serde_json::json!({ "status": "deleted" }));
        });

        let repo = repository(&server);
        let users = repo.fetch_users().await.unwrap();
        assert!(!users.pii_masked);
        assert_eq!(users.data.len(), 1);
        assert_eq!(users.data[0].id, "u1");

        let created = repo
            .invite_user(CreateUser {
                username: "new-user".into(),
                password: "password".into(),
                full_name: "New User".into(),
                email: "new@example.com".into(),
                role: "member".into(),
                is_system_admin: false,
            })
            .await
            .unwrap();
        assert_eq!(created.id, "u2");

        repo.reset_user_mfa("u1".into()).await.unwrap();
        repo.delete_user("u1".into(), false).await.unwrap();
        repo.unlock_user("u1".into()).await.unwrap();
        let archived = repo.fetch_archived_users().await.unwrap();
        assert_eq!(archived.len(), 1);
        repo.restore_archived_user("a1".into()).await.unwrap();
        repo.delete_archived_user("a1".into()).await.unwrap();
    }

    #[tokio::test]
    async fn invite_user_propagates_api_error() {
        let server = MockServer::start_async().await;
        server.mock(|when, then| {
            when.method(POST).path("/api/admin/users");
            then.status(409).json_body(serde_json::json!({
                "error": "username already exists",
                "code": "CONFLICT"
            }));
        });

        let repo = repository(&server);
        let error = repo
            .invite_user(CreateUser {
                username: "dup-user".into(),
                password: "password".into(),
                full_name: "Dup User".into(),
                email: "dup@example.com".into(),
                role: "member".into(),
                is_system_admin: false,
            })
            .await
            .expect_err("should return conflict");
        assert_eq!(error.code, "CONFLICT");
    }
}
