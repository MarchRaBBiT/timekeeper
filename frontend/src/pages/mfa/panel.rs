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

    let fetch_status = {
        create_action(move |_| async move {
            set_error.set(None);
            set_status_loading.set(true);
            match auth::fetch_mfa_status().await {
                Ok(resp) => set_status.set(Some(resp)),
                Err(err) => set_error.set(Some(err)),
            }
            set_status_loading.set(false);
        })
    };
    fetch_status.dispatch(());

    let register_action = {
        create_action(move |_| async move {
            let result = auth::register_mfa().await;
            if result.is_ok() {
                fetch_status.dispatch(());
            }
            result
        })
    };

    {
        create_effect(move |_| {
            if let Some(result) = register_action.value().get() {
                match result {
                    Ok(info) => {
                        set_setup_info.set(Some(info));
                        set_success.set(Some(
                            "認証アプリにシークレットを登録し、確認コードを入力してください。"
                                .into(),
                        ));
                        set_error.set(None);
                    }
                    Err(err) => set_error.set(Some(err)),
                }
            }
        });
    }

    let register_loading = register_action.pending();

    let activate_action = {
        create_action(move |code: &String| {
            let payload = code.clone();
            async move {
                let result = auth::activate_mfa(payload, Some(set_auth_state)).await;
                if result.is_ok() {
                    fetch_status.dispatch(());
                }
                result
            }
        })
    };

    {
        create_effect(move |_| {
            if let Some(result) = activate_action.value().get() {
                match result {
                    Ok(_) => {
                        set_setup_info.set(None);
                        set_totp_code.set(String::new());
                        set_success.set(Some(
                            "MFA を有効化しました。次回以降のログインで認証コードが求められます。"
                                .into(),
                        ));
                        set_error.set(None);
                    }
                    Err(err) => set_error.set(Some(err)),
                }
            }
        });
    }

    let activate_loading = activate_action.pending();

    let start_registration = {
        move || {
            if register_loading.get() {
                return;
            }
            set_error.set(None);
            set_success.set(None);
            set_setup_info.set(None);
            register_action.dispatch(());
        }
    };

    let handle_activate = {
        move |ev: SubmitEvent| {
            ev.prevent_default();
            if activate_loading.get() {
                return;
            }
            let code_value = totp_code.get();
            let trimmed = match utils::validate_totp_code(&code_value) {
                Ok(code) => code,
                Err(msg) => {
                    set_error.set(Some(msg));
                    return;
                }
            };

            set_error.set(None);
            set_success.set(None);
            activate_action.dispatch(trimmed);
        }
    };
    let handle_activate = Callback::new(handle_activate);

    view! {
        <Layout>
            <div class="mx-auto max-w-3xl space-y-6">
                <SetupSection
                    status=status
                    status_loading=status_loading
                    register_loading=register_loading.into()
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
                    activate_loading=activate_loading.into()
                    on_submit=handle_activate
                    on_input=set_totp_code
                />
            </div>
        </Layout>
    }
}
