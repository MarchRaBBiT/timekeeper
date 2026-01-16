use crate::{
    api::{ApiClient, ApiError, LoginRequest, UserResponse},
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

    let api_client = use_context::<ApiClient>().unwrap_or_else(ApiClient::new);
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
    use_context::<AuthContext>().unwrap_or_else(|| create_signal(AuthState::default()))
}

async fn check_auth_status(api_client: &ApiClient) -> Result<UserResponse, ApiError> {
    api_client.get_me().await
}

// TODO: リファクタリング後に使用可否を判断
// - 使う可能性: あり
// - 想定機能: セッション更新の手動実行
#[allow(dead_code)]
async fn refresh_session(api_client: &ApiClient) -> Result<UserResponse, ApiError> {
    let response = api_client.refresh_token().await?;
    Ok(response.user)
}

pub async fn login_request(
    request: LoginRequest,
    repo: &login_repository::LoginRepository,
    set_auth_state: WriteSignal<AuthState>,
) -> Result<(), ApiError> {
    set_auth_state.update(|state| state.loading = true);

    match repo.login(request).await {
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
    repo: &login_repository::LoginRepository,
    set_auth_state: WriteSignal<AuthState>,
) -> Result<(), ApiError> {
    let result = repo.logout(all_sessions).await;

    set_auth_state.update(|state| {
        state.user = None;
        state.is_authenticated = false;
        state.loading = false;
    });

    result
}

pub fn use_login_action() -> Action<LoginRequest, Result<(), ApiError>> {
    let (_auth, set_auth) = use_auth();
    let api = use_context::<ApiClient>().unwrap_or_else(ApiClient::new);
    let repo = login_repository::LoginRepository::new_with_client(std::rc::Rc::new(api));

    create_action(move |request: &LoginRequest| {
        let payload = request.clone();
        let repo = repo.clone();
        async move { login_request(payload, &repo, set_auth).await }
    })
}

pub fn use_logout_action() -> Action<bool, Result<(), ApiError>> {
    let (_auth, set_auth) = use_auth();
    let api = use_context::<ApiClient>().unwrap_or_else(ApiClient::new);
    let repo = login_repository::LoginRepository::new_with_client(std::rc::Rc::new(api));

    create_action(move |all: &bool| {
        let flag = *all;
        let repo = repo.clone();
        async move { logout(flag, &repo, set_auth).await }
    })
}
