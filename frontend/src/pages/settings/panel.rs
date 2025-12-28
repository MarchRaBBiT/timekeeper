use crate::{
    components::layout::{ErrorMessage, Layout, SuccessMessage},
    pages::{
        mfa::{
            components::{setup::SetupSection, verify::VerificationSection},
            utils,
        },
        settings::view_model::use_settings_view_model,
    },
};
use leptos::{ev::SubmitEvent, Callback, *};

fn map_change_password_error(error: &str) -> String {
    match error {
        "Current password is incorrect" => "現在のパスワードが正しくありません。".to_string(),
        "New password must be at least 8 characters" => {
            "新しいパスワードは8文字以上である必要があります。".to_string()
        }
        "New password must differ from current password" => {
            "新しいパスワードは現在のパスワードと異なる必要があります。".to_string()
        }
        _ => "パスワード変更に失敗しました。時間をおいて再度お試しください。".to_string(),
    }
}

#[component]
pub fn SettingsPage() -> impl IntoView {
    let vm = use_settings_view_model();

    // --- Password Change State ---
    let (current_password, set_current_password) = create_signal(String::new());
    let (new_password, set_new_password) = create_signal(String::new());
    let (confirm_password, set_confirm_password) = create_signal(String::new());

    let password_loading = vm.change_password_action.pending();
    let (password_success_msg, set_password_success_msg) = create_signal(Option::<String>::None);
    let (password_error_msg, set_password_error_msg) = create_signal(Option::<String>::None);

    create_effect(move |_| {
        if let Some(result) = vm.change_password_action.value().get() {
            match result {
                Ok(_) => {
                    set_password_success_msg.set(Some("パスワードを変更しました。".to_string()));
                    set_password_error_msg.set(None);
                    // Clear inputs
                    set_current_password.set(String::new());
                    set_new_password.set(String::new());
                    set_confirm_password.set(String::new());
                }
                Err(e) => {
                    set_password_error_msg.set(Some(map_change_password_error(&e)));
                    set_password_success_msg.set(None);
                }
            }
        }
    });

    let on_submit_password = move |ev: SubmitEvent| {
        ev.prevent_default();
        if password_loading.get() {
            return;
        }
        let current = current_password.get();
        let new = new_password.get();
        let confirm = confirm_password.get();

        if new.len() < 8 {
            set_password_error_msg.set(Some(
                "新しいパスワードは8文字以上である必要があります。".to_string(),
            ));
            return;
        }
        if new != confirm {
            set_password_error_msg.set(Some("新しいパスワードが一致しません。".to_string()));
            return;
        }

        set_password_error_msg.set(None);
        set_password_success_msg.set(None);
        vm.change_password_action.dispatch((current, new));
    };

    // --- MFA State (Reusing MfaViewModel) ---
    let mfa_vm = vm.mfa_view_model;
    let register_loading = mfa_vm.register_action.pending();
    let activate_loading = mfa_vm.activate_action.pending();

    // Logic adapted from MfaRegisterPanel for reuse
    let start_registration = {
        move || {
            if register_loading.get() {
                return;
            }
            mfa_vm.messages.clear();
            mfa_vm.setup_info.set(None);
            mfa_vm.register_action.dispatch(());
        }
    };

    let handle_activate = {
        move |ev: SubmitEvent| {
            ev.prevent_default();
            if activate_loading.get() {
                return;
            }
            let code_value = mfa_vm.totp_code.get();
            let trimmed = match utils::validate_totp_code(&code_value) {
                Ok(code) => code,
                Err(msg) => {
                    mfa_vm.messages.set_error(msg);
                    return;
                }
            };
            mfa_vm.messages.clear();
            mfa_vm.activate_action.dispatch(trimmed);
        }
    };
    let handle_activate_cb = Callback::new(handle_activate);

    view! {
        <Layout>
            <div class="mx-auto max-w-3xl space-y-8">

                // --- Password Change Section ---
                <div class="bg-white shadow rounded-lg p-6 space-y-4">
                    <h2 class="text-xl font-semibold text-gray-900 border-b pb-2">"パスワード変更"</h2>

                    <Show when=move || password_success_msg.get().is_some() fallback=|| ()>
                        <SuccessMessage message={password_success_msg.get().unwrap_or_default()} />
                    </Show>
                    <Show when=move || password_error_msg.get().is_some() fallback=|| ()>
                        <ErrorMessage message={password_error_msg.get().unwrap_or_default()} />
                    </Show>

                    <form class="space-y-4" on:submit=on_submit_password>
                        <div>
                            <label class="block text-sm font-medium text-gray-700">"現在のパスワード"</label>
                            <input type="password" required
                                class="mt-1 w-full border rounded px-3 py-2"
                                on:input=move |ev| set_current_password.set(event_target_value(&ev))
                                prop:value=current_password
                            />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700">"新しいパスワード"</label>
                            <input type="password" required
                                class="mt-1 w-full border rounded px-3 py-2"
                                on:input=move |ev| set_new_password.set(event_target_value(&ev))
                                prop:value=new_password
                            />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700">"新しいパスワード（確認）"</label>
                            <input type="password" required
                                class="mt-1 w-full border rounded px-3 py-2"
                                on:input=move |ev| set_confirm_password.set(event_target_value(&ev))
                                prop:value=confirm_password
                            />
                        </div>
                        <div class="flex justify-end">
                            <button type="submit"
                                class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50"
                                disabled=move || password_loading.get()
                            >
                                {move || if password_loading.get() { "変更中..." } else { "パスワードを変更" }}
                            </button>
                        </div>
                    </form>
                </div>

                // --- MFA Section ---
                // Reusing components from mfa page, but wrapped in our layout
                <div class="space-y-6">
                    <SetupSection
                        status=mfa_vm.status.read_only()
                        status_loading=mfa_vm.status_loading.read_only()
                        register_loading=register_loading.into()
                        on_register=start_registration
                        on_refresh=move || mfa_vm.fetch_status_action.dispatch(())
                    />
                    <Show when=move || mfa_vm.messages.success.get().is_some() fallback=|| ()>
                        <SuccessMessage message={mfa_vm.messages.success.get().unwrap_or_default()} />
                    </Show>
                    <Show when=move || mfa_vm.messages.error.get().is_some() fallback=|| ()>
                        <ErrorMessage message={mfa_vm.messages.error.get().unwrap_or_default()} />
                    </Show>
                    <VerificationSection
                        setup_info=mfa_vm.setup_info.read_only()
                        activate_loading=activate_loading.into()
                        on_submit=handle_activate_cb
                        on_input=mfa_vm.totp_code.write_only()
                    />
                </div>
            </div>
        </Layout>
    }
}

#[cfg(test)]
mod tests {
    use super::map_change_password_error;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn map_change_password_error_handles_known_messages() {
        assert_eq!(
            map_change_password_error("Current password is incorrect"),
            "現在のパスワードが正しくありません。"
        );
        assert_eq!(
            map_change_password_error("New password must be at least 8 characters"),
            "新しいパスワードは8文字以上である必要があります。"
        );
        assert_eq!(
            map_change_password_error("New password must differ from current password"),
            "新しいパスワードは現在のパスワードと異なる必要があります。"
        );
    }

    #[wasm_bindgen_test]
    fn map_change_password_error_masks_unknown_messages() {
        assert_eq!(
            map_change_password_error("Failed to update password"),
            "パスワード変更に失敗しました。時間をおいて再度お試しください。"
        );
    }
}
