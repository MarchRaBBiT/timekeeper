use super::{repository::MfaRepository, utils::MessageState};
use crate::{
    api::{ApiClient, MfaSetupResponse, MfaStatusResponse, ApiError},
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
