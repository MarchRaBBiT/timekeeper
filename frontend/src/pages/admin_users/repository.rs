use crate::api::{ApiClient, ApiError, ArchivedUserResponse, CreateUser, UserResponse};
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

    pub async fn fetch_users(&self) -> Result<Vec<UserResponse>, ApiError> {
        self.client.get_users().await
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

