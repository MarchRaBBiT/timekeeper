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

#[cfg(test)]
mod tests {
    use super::*;
    use leptos::create_runtime;

    fn with_runtime<T>(test: impl FnOnce() -> T) -> T {
        let runtime = create_runtime();
        let result = test();
        runtime.dispose();
        result
    }

    #[test]
    fn use_auth_returns_default_without_context() {
        with_runtime(|| {
            let (state, _set_state) = use_auth();
            let snapshot = state.get();
            assert!(!snapshot.is_authenticated);
            assert!(snapshot.user.is_none());
        });
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use httpmock::prelude::*;

    #[tokio::test]
    async fn login_and_logout_update_auth_state() {
        let server = MockServer::start_async().await;
        server.mock(|when, then| {
            when.method(POST).path("/api/auth/login");
            then.status(200).json_body(serde_json::json!({
                "user": {
                    "id": "u1",
                    "username": "alice",
                    "full_name": "Alice Example",
                    "role": "admin",
                    "is_system_admin": true,
                    "mfa_enabled": false
                }
            }));
        });
        server.mock(|when, then| {
            when.method(POST).path("/api/auth/logout");
            then.status(200).json_body(serde_json::json!({}));
        });

        let runtime = create_runtime();
        let (state, set_state) = create_signal(AuthState::default());
        let api = ApiClient::new_with_base_url(&server.url("/api"));
        let repo = login_repository::LoginRepository::new_with_client(std::rc::Rc::new(api));

        login_request(
            LoginRequest {
                username: "alice".into(),
                password: "secret".into(),
                totp_code: None,
                device_label: Some("test-device".into()),
            },
            &repo,
            set_state,
        )
        .await
        .unwrap();

        let snapshot = state.get();
        assert!(snapshot.is_authenticated);
        assert!(snapshot.user.is_some());

        logout(false, &repo, set_state).await.unwrap();
        let snapshot = state.get();
        assert!(!snapshot.is_authenticated);
        assert!(snapshot.user.is_none());
        runtime.dispose();
    }
}
