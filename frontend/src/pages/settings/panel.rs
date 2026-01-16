use crate::{
    api::{ApiError, CreateDataSubjectRequest, DataSubjectRequestType},
    components::{
        error::InlineErrorMessage,
        layout::{ErrorMessage, Layout, SuccessMessage},
    },
    pages::{
        mfa::{
            components::{setup::SetupSection, verify::VerificationSection},
            utils,
        },
        settings::view_model::use_settings_view_model,
    },
};
use chrono::{DateTime, Utc};
use leptos::{ev::SubmitEvent, Callback, *};

fn map_change_password_error(error: &ApiError) -> ApiError {
    match error.error.as_str() {
        "Current password is incorrect" => {
            ApiError::validation("現在のパスワードが正しくありません。")
        }
        "New password must be at least 8 characters" => {
            ApiError::validation("新しいパスワードは8文字以上である必要があります。")
        }
        "New password must differ from current password" => {
            ApiError::validation("新しいパスワードは現在のパスワードと異なる必要があります。")
        }
        _ => ApiError::unknown("パスワード変更に失敗しました。時間をおいて再度お試しください。"),
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
    let (password_error_msg, set_password_error_msg) = create_signal(Option::<ApiError>::None);

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
            set_password_error_msg.set(Some(ApiError::validation(
                "新しいパスワードは8文字以上である必要があります。",
            )));
            return;
        }
        if new != confirm {
            set_password_error_msg.set(Some(ApiError::validation(
                "新しいパスワードが一致しません。",
            )));
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

    // --- Subject Request State ---
    let subject_vm = vm.subject_request_view_model;
    let subject_request_type = create_rw_signal("access".to_string());
    let subject_details = create_rw_signal(String::new());
    let subject_loading = subject_vm.create_action.pending();
    let cancel_loading = subject_vm.cancel_action.pending();
    let cancel_action = subject_vm.cancel_action;
    let subject_requests_resource = subject_vm.requests_resource;
    let subject_requests_error = Signal::derive(move || {
        subject_requests_resource
            .get()
            .and_then(|res| res.err())
            .map(|err| err.to_string())
    });
    let subject_requests = Signal::derive(move || {
        subject_requests_resource
            .get()
            .and_then(|res| res.ok())
            .unwrap_or_default()
    });
    let (subject_success_msg, set_subject_success_msg) = create_signal(Option::<String>::None);
    let (subject_error_msg, set_subject_error_msg) = create_signal(Option::<String>::None);

    create_effect(move |_| {
        if let Some(result) = subject_vm.create_action.value().get() {
            match result {
                Ok(_) => {
                    set_subject_success_msg.set(Some("本人対応申請を送信しました。".into()));
                    set_subject_error_msg.set(None);
                    subject_details.set(String::new());
                    subject_vm
                        .reload
                        .update(|value| *value = value.wrapping_add(1));
                }
                Err(err) => {
                    set_subject_error_msg.set(Some(err.to_string()));
                    set_subject_success_msg.set(None);
                }
            }
        }
    });

    create_effect(move |_| {
        if let Some(result) = subject_vm.cancel_action.value().get() {
            match result {
                Ok(_) => {
                    set_subject_success_msg.set(Some("本人対応申請を取消しました。".into()));
                    set_subject_error_msg.set(None);
                    subject_vm
                        .reload
                        .update(|value| *value = value.wrapping_add(1));
                }
                Err(err) => {
                    set_subject_error_msg.set(Some(err.to_string()));
                    set_subject_success_msg.set(None);
                }
            }
        }
    });

    let on_submit_subject = move |ev: SubmitEvent| {
        ev.prevent_default();
        if subject_loading.get() {
            return;
        }
        let request_type = match subject_request_type.get().as_str() {
            "access" => DataSubjectRequestType::Access,
            "rectify" => DataSubjectRequestType::Rectify,
            "delete" => DataSubjectRequestType::Delete,
            "stop" => DataSubjectRequestType::Stop,
            _ => {
                set_subject_error_msg.set(Some("申請種別を選択してください。".into()));
                return;
            }
        };
        let details_raw = subject_details.get();
        let details = if details_raw.trim().is_empty() {
            None
        } else {
            Some(details_raw.trim().to_string())
        };
        set_subject_error_msg.set(None);
        set_subject_success_msg.set(None);
        subject_vm.create_action.dispatch(CreateDataSubjectRequest {
            request_type,
            details,
        });
    };

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
                        <InlineErrorMessage error={password_error_msg.into()} />
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
                        <InlineErrorMessage error={mfa_vm.messages.error.into()} />
                    </Show>
                    <VerificationSection
                        setup_info=mfa_vm.setup_info.read_only()
                        activate_loading=activate_loading.into()
                        on_submit=handle_activate_cb
                        on_input=mfa_vm.totp_code.write_only()
                    />
                </div>

                // --- Subject Request Section ---
                <div class="bg-white shadow rounded-lg p-6 space-y-4">
                    <h2 class="text-xl font-semibold text-gray-900 border-b pb-2">"本人対応申請"</h2>
                    <Show when=move || subject_success_msg.get().is_some() fallback=|| ()>
                        <SuccessMessage message={subject_success_msg.get().unwrap_or_default()} />
                    </Show>
                    <Show when=move || subject_error_msg.get().is_some() fallback=|| ()>
                        <ErrorMessage message={subject_error_msg.get().unwrap_or_default()} />
                    </Show>
                    <form class="space-y-3" on:submit=on_submit_subject>
                        <div>
                            <label class="block text-sm font-medium text-gray-700">"申請種別"</label>
                            <select
                                class="mt-1 w-full border rounded px-3 py-2"
                                prop:value={move || subject_request_type.get()}
                                on:change=move |ev| subject_request_type.set(event_target_value(&ev))
                            >
                                <option value="access">{"開示"}</option>
                                <option value="rectify">{"訂正"}</option>
                                <option value="delete">{"削除"}</option>
                                <option value="stop">{"停止"}</option>
                            </select>
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700">"詳細"</label>
                            <textarea
                                class="mt-1 w-full border rounded px-3 py-2"
                                rows="3"
                                prop:value={move || subject_details.get()}
                                on:input=move |ev| subject_details.set(event_target_value(&ev))
                            ></textarea>
                        </div>
                        <div class="flex justify-end">
                            <button
                                type="submit"
                                class="px-4 py-2 bg-blue-600 text-white rounded disabled:opacity-50"
                                disabled=move || subject_loading.get()
                            >
                                {move || if subject_loading.get() { "送信中..." } else { "申請する" }}
                            </button>
                        </div>
                    </form>
                    <div>
                        <h3 class="text-sm font-medium text-gray-700 mb-2">{"申請履歴"}</h3>
                        <Show when=move || subject_requests_error.get().is_some()>
                            <ErrorMessage message={subject_requests_error.get().unwrap_or_default()} />
                        </Show>
                        <div class="overflow-x-auto">
                            <table class="min-w-full divide-y divide-gray-200">
                                <thead class="bg-gray-50">
                                    <tr>
                                        <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"種別"}</th>
                                        <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"ステータス"}</th>
                                        <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"申請日"}</th>
                                        <th class="px-4 py-2 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">{"操作"}</th>
                                    </tr>
                                </thead>
                                <tbody class="bg-white divide-y divide-gray-200">
                                    {move || {
                                        subject_requests
                                            .get()
                                            .into_iter()
                                            .map(|request| {
                                                let can_cancel = request.status == "pending";
                                                let type_label = subject_request_type_label(&request.request_type);
                                                let status_label = subject_request_status_label(&request.status);
                                                let created_label = format_subject_datetime(request.created_at);
                                                let request_id = request.id.clone();
                                                let on_cancel = {
                                                    let request_id = request.id.clone();
                                                    move |_| cancel_action.dispatch(request_id.clone())
                                                };
                                                view! {
                                                    <tr>
                                                        <td class="px-4 py-2 whitespace-nowrap text-sm text-gray-900">{type_label}</td>
                                                        <td class="px-4 py-2 whitespace-nowrap text-sm text-gray-700">{status_label}</td>
                                                        <td class="px-4 py-2 whitespace-nowrap text-sm text-gray-700">{created_label}</td>
                                                        <td class="px-4 py-2 whitespace-nowrap text-right text-sm">
                                                            <button
                                                                class="text-red-600 disabled:opacity-50"
                                                                disabled={move || cancel_loading.get() || !can_cancel}
                                                                on:click=on_cancel
                                                            >
                                                                {"取消"}
                                                            </button>
                                                            <span class="sr-only">{request_id}</span>
                                                        </td>
                                                    </tr>
                                                }
                                            })
                                            .collect::<Vec<_>>()
                                    }}
                                </tbody>
                            </table>
                        </div>
                    </div>
                </div>
            </div>
        </Layout>
    }
}

fn subject_request_type_label(value: &DataSubjectRequestType) -> &'static str {
    match value {
        DataSubjectRequestType::Access => "開示",
        DataSubjectRequestType::Rectify => "訂正",
        DataSubjectRequestType::Delete => "削除",
        DataSubjectRequestType::Stop => "停止",
    }
}

fn subject_request_status_label(value: &str) -> String {
    match value {
        "pending" => "承認待ち".to_string(),
        "approved" => "承認済み".to_string(),
        "rejected" => "却下".to_string(),
        "cancelled" => "取消".to_string(),
        _ => value.to_string(),
    }
}

fn format_subject_datetime(value: DateTime<Utc>) -> String {
    value.format("%Y-%m-%d %H:%M").to_string()
}

#[cfg(test)]
mod tests {
    use super::map_change_password_error;
    use crate::api::ApiError;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn map_change_password_error_handles_known_messages() {
        assert_eq!(
            map_change_password_error(&ApiError::unknown("Current password is incorrect")).error,
            "現在のパスワードが正しくありません。"
        );
        assert_eq!(
            map_change_password_error(&ApiError::unknown(
                "New password must be at least 8 characters"
            ))
            .error,
            "新しいパスワードは8文字以上である必要があります。"
        );
        assert_eq!(
            map_change_password_error(&ApiError::unknown(
                "New password must differ from current password"
            ))
            .error,
            "新しいパスワードは現在のパスワードと異なる必要があります。"
        );
    }

    #[wasm_bindgen_test]
    fn map_change_password_error_masks_unknown_messages() {
        assert_eq!(
            map_change_password_error(&ApiError::unknown("Failed to update password")).error,
            "パスワード変更に失敗しました。時間をおいて再度お試しください。"
        );
    }
}
