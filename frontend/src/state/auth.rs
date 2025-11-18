#![allow(dead_code)]
use crate::{
    api::{ApiClient, LoginRequest, MfaSetupResponse, MfaStatusResponse, UserResponse},
    utils::storage as storage_utils,
};
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
    let storage = storage_utils::local_storage()?;
    let token = storage
        .get_item("access_token")
        .map_err(|_| "Failed to get token")?;

    if token.as_deref().map(|t| t.is_empty()).unwrap_or(true) {
        return refresh_session(api_client).await;
    }

    match storage
        .get_item("current_user")
        .map_err(|_| "Failed to read user profile")?
    {
        Some(user_json) => serde_json::from_str(&user_json)
            .map_err(|_| "Failed to parse stored user profile".to_string()),
        None => refresh_session(api_client).await,
    }
}

async fn refresh_session(api_client: &ApiClient) -> Result<UserResponse, String> {
    let response = api_client.refresh_token().await?;
    Ok(response.user)
}

pub async fn login(
    username: String,
    password: String,
    totp_code: Option<String>,
    set_auth_state: WriteSignal<AuthState>,
) -> Result<(), String> {
    set_auth_state.update(|state| state.loading = true);

    let api_client = ApiClient::new();
    let request = LoginRequest {
        username,
        password,
        totp_code,
        device_label: None,
    };

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
    if let Ok(storage) = storage_utils::local_storage() {
        let _ = storage.remove_item("access_token");
        let _ = storage.remove_item("access_token_jti");
        let _ = storage.remove_item("refresh_token");
        let _ = storage.remove_item("current_user");
    }

    set_auth_state.update(|state| {
        state.user = None;
        state.is_authenticated = false;
        state.loading = false;
    });
}

pub async fn fetch_mfa_status() -> Result<MfaStatusResponse, String> {
    let api_client = ApiClient::new();
    api_client.get_mfa_status().await
}

pub async fn register_mfa() -> Result<MfaSetupResponse, String> {
    let api_client = ApiClient::new();
    api_client.register_mfa().await
}

pub async fn activate_mfa(
    code: String,
    set_auth_state: Option<WriteSignal<AuthState>>,
) -> Result<(), String> {
    let api_client = ApiClient::new();
    api_client.activate_mfa(&code).await?;

    if let Ok(storage) = storage_utils::local_storage() {
        if let Ok(Some(user_json)) = storage.get_item("current_user") {
            if let Ok(mut user) = serde_json::from_str::<UserResponse>(&user_json) {
                if !user.mfa_enabled {
                    user.mfa_enabled = true;
                    if let Ok(updated) = serde_json::to_string(&user) {
                        let _ = storage.set_item("current_user", &updated);
                    }
                }
            }
        }
    }

    if let Some(setter) = set_auth_state {
        setter.update(|state| {
            if let Some(user) = state.user.as_mut() {
                user.mfa_enabled = true;
            }
        });
    }

    Ok(())
}
