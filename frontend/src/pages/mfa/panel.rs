use crate::{
    components::layout::{ErrorMessage, Layout, SuccessMessage},
    pages::mfa::{
        components::{setup::SetupSection, verify::VerificationSection},
        utils,
    },
    state::auth::{self, use_auth},
};
use leptos::{ev::SubmitEvent, Callback, *};

#[component]
pub fn MfaRegisterPanel() -> impl IntoView {
    let (_auth_state, set_auth_state) = use_auth();

    let (status, set_status) = create_signal(None);
    let (status_loading, set_status_loading) = create_signal(true);
    let (error, set_error) = create_signal(None::<String>);
    let (success, set_success) = create_signal(None::<String>);
    let (setup_info, set_setup_info) = create_signal(None);
    let (totp_code, set_totp_code) = create_signal(String::new());
    let (register_loading, set_register_loading) = create_signal(false);
    let (activate_loading, set_activate_loading) = create_signal(false);

    let fetch_status = {
        let set_status = set_status.clone();
        let set_status_loading = set_status_loading.clone();
        let set_error = set_error.clone();
        create_action(move |_| {
            let set_status = set_status.clone();
            let set_status_loading = set_status_loading.clone();
            let set_error = set_error.clone();
            async move {
                set_error.set(None);
                set_status_loading.set(true);
                match auth::fetch_mfa_status().await {
                    Ok(resp) => set_status.set(Some(resp)),
                    Err(err) => set_error.set(Some(err)),
                }
                set_status_loading.set(false);
            }
        })
    };
    fetch_status.dispatch(());

    let start_registration = {
        let fetch_status_action = fetch_status.clone();
        let set_error = set_error.clone();
        let set_success = set_success.clone();
        let set_setup_info = set_setup_info.clone();
        let set_register_loading = set_register_loading.clone();
        move || {
            if register_loading.get() {
                return;
            }
            set_error.set(None);
            set_success.set(None);
            set_setup_info.set(None);
            set_register_loading.set(true);
            let fetch_status_action = fetch_status_action.clone();
            let set_setup_info = set_setup_info.clone();
            let set_success = set_success.clone();
            let set_error = set_error.clone();
            let set_register_loading = set_register_loading.clone();
            spawn_local(async move {
                match auth::register_mfa().await {
                    Ok(info) => {
                        set_setup_info.set(Some(info));
                        fetch_status_action.dispatch(());
                        set_success.set(Some(
                            "認証アプリにシークレットを登録し、確認コードを入力してください。"
                                .into(),
                        ));
                    }
                    Err(err) => set_error.set(Some(err)),
                }
                set_register_loading.set(false);
            });
        }
    };

    let handle_activate = {
        let fetch_status_action = fetch_status.clone();
        let totp_code_signal = totp_code.clone();
        let set_totp_code = set_totp_code.clone();
        let set_setup_info = set_setup_info.clone();
        let set_error = set_error.clone();
        let set_success = set_success.clone();
        let set_activate_loading = set_activate_loading.clone();
        let set_auth_state = set_auth_state.clone();
        move |ev: SubmitEvent| {
            ev.prevent_default();
            if activate_loading.get() {
                return;
            }
            let code_value = totp_code_signal.get();

            let trimmed = match utils::validate_totp_code(&code_value) {
                Ok(code) => code,

                Err(msg) => {
                    set_error.set(Some(msg));

                    return;
                }
            };

            set_activate_loading.set(true);
            set_error.set(None);
            set_success.set(None);

            let fetch_status_action = fetch_status_action.clone();
            let set_setup_info = set_setup_info.clone();
            let set_totp_code = set_totp_code.clone();
            let set_success = set_success.clone();
            let set_error = set_error.clone();
            let set_activate_loading = set_activate_loading.clone();
            let set_auth_state = set_auth_state.clone();
            spawn_local(async move {
                match auth::activate_mfa(trimmed, Some(set_auth_state)).await {
                    Ok(_) => {
                        set_setup_info.set(None);
                        set_totp_code.set(String::new());
                        fetch_status_action.dispatch(());
                        set_success.set(Some("MFA を有効化しました。次回以降のログインにワンタイムコードが必要です。".into()));
                    }
                    Err(err) => set_error.set(Some(err)),
                }
                set_activate_loading.set(false);
            });
        }
    };
    let handle_activate = Callback::new(handle_activate);

    view! {
        <Layout>
            <div class="mx-auto max-w-3xl space-y-6">
                <SetupSection
                    status=status
                    status_loading=status_loading
                    register_loading=register_loading
                    on_register=start_registration
                    on_refresh=move || fetch_status.dispatch(())
                />
                <Show when=move || success.get().is_some() fallback=|| ()>
                    <SuccessMessage message={success.get().unwrap_or_default()} />
                </Show>
                <Show when=move || error.get().is_some() fallback=|| ()>
                    <ErrorMessage message={error.get().unwrap_or_default()} />
                </Show>
                <VerificationSection
                    setup_info=setup_info
                    activate_loading=activate_loading
                    on_submit=handle_activate
                    on_input=set_totp_code
                />
            </div>
        </Layout>
    }
}
