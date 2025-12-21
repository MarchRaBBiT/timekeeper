use crate::api::{ApiClient, CreateUser, UserResponse};
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
            client: Rc::new(ApiClient::new()),
        }
    }

    pub async fn fetch_users(&self) -> Result<Vec<UserResponse>, String> {
        self.client.get_users().await
    }

    pub async fn invite_user(&self, payload: CreateUser) -> Result<UserResponse, String> {
        self.client.create_user(payload).await
    }

    pub async fn reset_user_mfa(&self, user_id: String) -> Result<(), String> {
        self.client.admin_reset_mfa(&user_id).await
    }
}
