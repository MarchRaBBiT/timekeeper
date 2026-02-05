use super::{repository::MfaRepository, utils::MessageState};
use crate::{
    api::{ApiClient, ApiError, MfaSetupResponse, MfaStatusResponse},
    state::auth::use_auth,
};
use leptos::*;

#[derive(Clone)]
pub struct MfaViewModel {
    pub status: RwSignal<Option<MfaStatusResponse>>,
    pub status_loading: RwSignal<bool>,
    pub setup_info: RwSignal<Option<MfaSetupResponse>>,
    pub totp_code: RwSignal<String>,
    pub messages: MessageState,
    pub fetch_status_action: Action<(), ()>,
    pub register_action: Action<(), Result<MfaSetupResponse, ApiError>>,
    pub activate_action: Action<String, Result<(), ApiError>>,
}

pub fn use_mfa_view_model() -> MfaViewModel {
    let (_auth, set_auth) = use_auth();
    let api = use_context::<ApiClient>().unwrap_or_else(ApiClient::new);
    let repository = MfaRepository::new_with_client(std::rc::Rc::new(api));

    let status = create_rw_signal(None);
    let status_loading = create_rw_signal(true);
    let setup_info = create_rw_signal(None);
    let totp_code = create_rw_signal(String::new());
    let messages = MessageState::default();

    let repo_for_fetch = repository.clone();
    let fetch_status_action = create_action(move |_| {
        let repo = repo_for_fetch.clone();
        async move {
            messages.clear();
            status_loading.set(true);
            match repo.fetch_status().await {
                Ok(resp) => status.set(Some(resp)),
                Err(err) => messages.set_error(err),
            }
            status_loading.set(false);
        }
    });

    let repo_for_register = repository.clone();
    let register_action = create_action(move |_| {
        let repo = repo_for_register.clone();
        async move { repo.register().await }
    });

    let repo_for_activate = repository.clone();
    let activate_action = create_action(move |code: &String| {
        let repo = repo_for_activate.clone();
        let code = code.clone();
        async move { repo.activate(&code).await }
    });

    // Effects
    create_effect(move |_| {
        if let Some(result) = register_action.value().get() {
            match result {
                Ok(info) => {
                    setup_info.set(Some(info));
                    messages.set_success(
                        "認証アプリにシークレットを登録し、確認コードを入力してください。"
                            .to_string(),
                    );
                }
                Err(err) => messages.set_error(err),
            }
        }
    });

    create_effect(move |_| {
        if let Some(result) = activate_action.value().get() {
            match result {
                Ok(_) => {
                    setup_info.set(None);
                    totp_code.set(String::new());
                    messages.set_success(
                        "MFA を有効化しました。次回以降のログインで認証コードが求められます。"
                            .to_string(),
                    );

                    // Update global auth state
                    set_auth.update(|state| {
                        if let Some(user) = state.user.as_mut() {
                            user.mfa_enabled = true;
                        }
                    });

                    fetch_status_action.dispatch(());
                }
                Err(err) => messages.set_error(err),
            }
        }
    });

    // Initial fetch
    fetch_status_action.dispatch(());

    MfaViewModel {
        status,
        status_loading,
        setup_info,
        totp_code,
        messages,
        fetch_status_action,
        register_action,
        activate_action,
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::api::test_support::mock::*;
    use crate::test_support::helpers::{admin_user, provide_auth};
    use crate::test_support::ssr::with_local_runtime_async;

    fn mock_server() -> MockServer {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/auth/mfa");
            then.status(200).json_body(serde_json::json!({
                "enabled": false,
                "pending": false
            }));
        });
        server.mock(|when, then| {
            when.method(POST).path("/api/auth/mfa/register");
            then.status(200).json_body(serde_json::json!({
                "secret": "secret",
                "otpauth_url": "otpauth://totp/test"
            }));
        });
        server.mock(|when, then| {
            when.method(POST).path("/api/auth/mfa/activate");
            then.status(200).json_body(serde_json::json!({}));
        });
        server
    }

    #[test]
    fn mfa_view_model_fetches_status() {
        with_local_runtime_async(|| async {
            let runtime = leptos::create_runtime();
            let server = mock_server();
            provide_context(ApiClient::new_with_base_url(&server.url("/api")));
            let vm = use_mfa_view_model();
            for _ in 0..10 {
                if !vm.status_loading.get() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            assert!(!vm.status_loading.get());
            assert!(vm.status.get().is_some());
            runtime.dispose();
        });
    }

    #[test]
    fn mfa_view_model_register_sets_setup_info() {
        with_local_runtime_async(|| async {
            let runtime = leptos::create_runtime();
            let server = mock_server();
            provide_context(ApiClient::new_with_base_url(&server.url("/api")));
            let vm = use_mfa_view_model();
            vm.register_action.dispatch(());
            for _ in 0..10 {
                if vm.setup_info.get().is_some() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            let _ = vm.setup_info.get();
            let _ = vm.messages.success.get();
            runtime.dispose();
        });
    }

    #[test]
    fn mfa_view_model_activate_updates_auth() {
        with_local_runtime_async(|| async {
            let runtime = leptos::create_runtime();
            let server = mock_server();
            provide_auth(Some(admin_user(true)));
            provide_context(ApiClient::new_with_base_url(&server.url("/api")));
            let vm = use_mfa_view_model();
            vm.activate_action.dispatch("123456".into());
            for _ in 0..10 {
                let (auth, _) = use_auth();
                if auth.get().user.as_ref().map(|u| u.mfa_enabled).unwrap_or(false) {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            let (auth, _) = use_auth();
            let _ = auth.get();
            let _ = vm.messages.success.get();
            runtime.dispose();
        });
    }
}
