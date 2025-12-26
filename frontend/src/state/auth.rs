use crate::{
    api::{ApiClient, LoginRequest, MfaSetupResponse, MfaStatusResponse, UserResponse},
    pages::login::repository as login_repository,
};
use leptos::*;

type AuthContext = (ReadSignal<AuthState>, WriteSignal<AuthState>);

#[derive(Debug, Clone, Default)]
pub struct AuthState {
    pub user: Option<UserResponse>,
    pub is_authenticated: bool,
    pub loading: bool,
}

fn create_auth_context() -> AuthContext {
    let (auth_state, set_auth_state) = create_signal(AuthState::default());
    set_auth_state.update(|state| state.loading = true);

    let api_client = use_context::<ApiClient>().expect("ApiClient should be provided");
    let set_auth_for_check = set_auth_state;
    spawn_local(async move {
        match check_auth_status(&api_client).await {
            Ok(user) => set_auth_for_check.update(|state| {
                state.user = Some(user);
                state.is_authenticated = true;
                state.loading = false;
            }),
            Err(_) => set_auth_for_check.update(|state| {
                state.user = None;
                state.is_authenticated = false;
                state.loading = false;
            }),
        }
    });

    (auth_state, set_auth_state)
}

#[component]
pub fn AuthProvider(children: Children) -> impl IntoView {
    let ctx = create_auth_context();
    provide_context::<AuthContext>(ctx);
    view! { <>{children()}</> }
}

pub fn use_auth() -> AuthContext {
    use_context::<AuthContext>().expect("AuthProvider がマウントされていません")
}

async fn check_auth_status(api_client: &ApiClient) -> Result<UserResponse, String> {
    api_client.get_me().await
}

async fn refresh_session(api_client: &ApiClient) -> Result<UserResponse, String> {
    let response = api_client.refresh_token().await?;
    Ok(response.user)
}

pub async fn login_request(
    request: LoginRequest,
    set_auth_state: WriteSignal<AuthState>,
) -> Result<(), String> {
    set_auth_state.update(|state| state.loading = true);

    match login_repository::login(request).await {
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

pub async fn logout(
    all_sessions: bool,
    set_auth_state: WriteSignal<AuthState>,
) -> Result<(), String> {
    let result = login_repository::logout(all_sessions).await;

    set_auth_state.update(|state| {
        state.user = None;
        state.is_authenticated = false;
        state.loading = false;
    });

    result
}

pub async fn fetch_mfa_status() -> Result<MfaStatusResponse, String> {
    let api = use_context::<ApiClient>().expect("ApiClient should be provided");
    api.get_mfa_status().await
}

pub async fn register_mfa() -> Result<MfaSetupResponse, String> {
    let api = use_context::<ApiClient>().expect("ApiClient should be provided");
    api.register_mfa().await
}

pub async fn activate_mfa(
    code: String,
    set_auth_state: Option<WriteSignal<AuthState>>,
) -> Result<(), String> {
    let api = use_context::<ApiClient>().expect("ApiClient should be provided");
    api.activate_mfa(&code).await?;

    if let Some(setter) = set_auth_state {
        setter.update(|state| {
            if let Some(user) = state.user.as_mut() {
                user.mfa_enabled = true;
            }
        });
    }

    Ok(())
}

pub fn use_login_action() -> Action<LoginRequest, Result<(), String>> {
    let (_auth, set_auth) = use_auth();
    create_action(move |request: &LoginRequest| {
        let payload = request.clone();
        async move { login_request(payload, set_auth).await }
    })
}

pub fn use_logout_action() -> Action<bool, Result<(), String>> {
    let (_auth, set_auth) = use_auth();
    create_action(move |all: &bool| {
        let flag = *all;
        async move { logout(flag, set_auth).await }
    })
}
