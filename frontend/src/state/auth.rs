#![allow(dead_code)]
use crate::api::{ApiClient, LoginRequest, UserResponse};
use leptos::*;

#[derive(Debug, Clone)]
pub struct AuthState {
    pub user: Option<UserResponse>,
    pub is_authenticated: bool,
    pub loading: bool,
}

impl Default for AuthState {
    fn default() -> Self {
        Self {
            user: None,
            is_authenticated: false,
            loading: false,
        }
    }
}

pub fn use_auth() -> (ReadSignal<AuthState>, WriteSignal<AuthState>) {
    let (auth_state, set_auth_state) = create_signal(AuthState::default());

    // Check if user is already logged in on mount
    let api_client = ApiClient::new();
    spawn_local(async move {
        if let Ok(user) = check_auth_status(&api_client).await {
            set_auth_state.update(|state| {
                state.user = Some(user);
                state.is_authenticated = true;
            });
        }
    });

    (auth_state, set_auth_state)
}

async fn check_auth_status(_api_client: &ApiClient) -> Result<UserResponse, String> {
    // Try to get user info by making a request to a protected endpoint
    // For now, we'll just check if token exists in localStorage
    let window = web_sys::window().ok_or("No window object")?;
    let storage = window
        .local_storage()
        .map_err(|_| "No localStorage")?
        .ok_or("No localStorage")?;
    let token = storage
        .get_item("access_token")
        .map_err(|_| "Failed to get token")?
        .ok_or("No token")?;

    if token.is_empty() {
        return Err("No token".to_string());
    }

    let user_json = storage
        .get_item("current_user")
        .map_err(|_| "Failed to read user profile")?
        .ok_or("No stored user profile")?;

    serde_json::from_str(&user_json).map_err(|_| "Failed to parse stored user profile".to_string())
}

pub async fn login(
    username: String,
    password: String,
    set_auth_state: WriteSignal<AuthState>,
) -> Result<(), String> {
    set_auth_state.update(|state| state.loading = true);

    let api_client = ApiClient::new();
    let request = LoginRequest { username, password };

    match api_client.login(request).await {
        Ok(response) => {
            set_auth_state.update(|state| {
                state.user = Some(response.user);
                state.is_authenticated = true;
                state.loading = false;
            });
            Ok(())
        }
        Err(error) => {
            set_auth_state.update(|state| state.loading = false);
            Err(error)
        }
    }
}

pub async fn logout(set_auth_state: WriteSignal<AuthState>) {
    // Try to notify backend to revoke refresh token (best-effort)
    let api_client = ApiClient::new();
    let _ = api_client.logout(false).await;

    // Clear tokens from localStorage regardless of backend result
    let window = web_sys::window().unwrap();
    if let Ok(Some(storage)) = window.local_storage() {
        let _ = storage.remove_item("access_token");
        let _ = storage.remove_item("refresh_token");
        let _ = storage.remove_item("current_user");
    }

    set_auth_state.update(|state| {
        state.user = None;
        state.is_authenticated = false;
        state.loading = false;
    });
}
