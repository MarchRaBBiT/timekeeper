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

fn validate_password_submission(
    new_password: &str,
    confirm_password: &str,
) -> Result<(), ApiError> {
    if new_password.len() < 8 {
        return Err(ApiError::validation(
            "新しいパスワードは8文字以上である必要があります。",
        ));
    }
    if new_password != confirm_password {
        return Err(ApiError::validation("新しいパスワードが一致しません。"));
    }
    Ok(())
}

fn parse_subject_request_type(value: &str) -> Result<DataSubjectRequestType, &'static str> {
    match value {
        "access" => Ok(DataSubjectRequestType::Access),
        "rectify" => Ok(DataSubjectRequestType::Rectify),
        "delete" => Ok(DataSubjectRequestType::Delete),
        "stop" => Ok(DataSubjectRequestType::Stop),
        _ => Err("申請種別を選択してください。"),
    }
}

fn normalize_subject_details(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn prepare_password_change_submission(
    pending: bool,
    current: String,
    new_password: String,
    confirm_password: String,
) -> Result<Option<(String, String)>, ApiError> {
    if pending {
        return Ok(None);
    }

    validate_password_submission(&new_password, &confirm_password)?;
    Ok(Some((current, new_password)))
}

fn prepare_mfa_activation_submission(
    pending: bool,
    code: &str,
) -> Result<Option<String>, ApiError> {
    if pending {
        return Ok(None);
    }
    let trimmed = utils::validate_totp_code(code)?;
    Ok(Some(trimmed))
}

fn prepare_subject_request_submission(
    pending: bool,
    request_type: &str,
    details: &str,
) -> Result<Option<CreateDataSubjectRequest>, String> {
    if pending {
        return Ok(None);
    }

    let parsed_type = parse_subject_request_type(request_type).map_err(|msg| msg.to_string())?;
    Ok(Some(CreateDataSubjectRequest {
        request_type: parsed_type,
        details: normalize_subject_details(details),
    }))
}

fn password_change_feedback(result: Result<(), ApiError>) -> (Option<String>, Option<ApiError>) {
    match result {
        Ok(_) => (Some("パスワードを変更しました。".to_string()), None),
        Err(err) => (None, Some(map_change_password_error(&err))),
    }
}

fn subject_create_feedback<T>(result: Result<T, ApiError>) -> (Option<String>, Option<String>) {
    match result {
        Ok(_) => (Some("本人対応申請を送信しました。".into()), None),
        Err(err) => (None, Some(err.to_string())),
    }
}

fn subject_cancel_feedback(result: Result<(), ApiError>) -> (Option<String>, Option<String>) {
    match result {
        Ok(_) => (Some("本人対応申請を取消しました。".into()), None),
        Err(err) => (None, Some(err.to_string())),
    }
}

fn is_subject_cancel_disabled(cancel_loading: bool, can_cancel: bool) -> bool {
    cancel_loading || !can_cancel
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
            let (success_msg, error_msg) = password_change_feedback(result);
            set_password_success_msg.set(success_msg);
            set_password_error_msg.set(error_msg);
            if password_error_msg.get_untracked().is_none() {
                // Clear inputs only on success.
                set_current_password.set(String::new());
                set_new_password.set(String::new());
                set_confirm_password.set(String::new());
            }
        }
    });

    let on_submit_password = move |ev: SubmitEvent| {
        ev.prevent_default();
        match prepare_password_change_submission(
            password_loading.get(),
            current_password.get(),
            new_password.get(),
            confirm_password.get(),
        ) {
            Ok(Some((current, new))) => {
                set_password_error_msg.set(None);
                set_password_success_msg.set(None);
                vm.change_password_action.dispatch((current, new));
            }
            Ok(None) => {}
            Err(err) => set_password_error_msg.set(Some(err)),
        }
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
            match prepare_mfa_activation_submission(activate_loading.get(), &mfa_vm.totp_code.get())
            {
                Ok(Some(trimmed)) => {
                    mfa_vm.messages.clear();
                    mfa_vm.activate_action.dispatch(trimmed);
                }
                Ok(None) => {}
                Err(msg) => mfa_vm.messages.set_error(msg),
            }
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
            let (success_msg, error_msg) = subject_create_feedback(result);
            let should_reload = error_msg.is_none();
            set_subject_success_msg.set(success_msg);
            set_subject_error_msg.set(error_msg);
            if should_reload {
                subject_details.set(String::new());
                subject_vm
                    .reload
                    .update(|value| *value = value.wrapping_add(1));
            }
        }
    });

    create_effect(move |_| {
        if let Some(result) = subject_vm.cancel_action.value().get() {
            let (success_msg, error_msg) = subject_cancel_feedback(result);
            let should_reload = error_msg.is_none();
            set_subject_success_msg.set(success_msg);
            set_subject_error_msg.set(error_msg);
            if should_reload {
                subject_vm
                    .reload
                    .update(|value| *value = value.wrapping_add(1));
            }
        }
    });

    let on_submit_subject = move |ev: SubmitEvent| {
        ev.prevent_default();
        match prepare_subject_request_submission(
            subject_loading.get(),
            subject_request_type.get().as_str(),
            &subject_details.get(),
        ) {
            Ok(Some(payload)) => {
                set_subject_error_msg.set(None);
                set_subject_success_msg.set(None);
                subject_vm.create_action.dispatch(payload);
            }
            Ok(None) => {}
            Err(msg) => set_subject_error_msg.set(Some(msg)),
        }
    };

    view! {
        <Layout>
            <div class="mx-auto max-w-3xl space-y-8">

                // --- Password Change Section ---
                <div class="bg-surface-elevated shadow rounded-lg p-6 space-y-4">
                    <h2 class="text-xl font-semibold text-fg border-b border-border pb-2">"パスワード変更"</h2>

                    <Show when=move || password_success_msg.get().is_some() fallback=|| ()>
                        <SuccessMessage message={password_success_msg.get().unwrap_or_default()} />
                    </Show>
                    <Show when=move || password_error_msg.get().is_some() fallback=|| ()>
                        <InlineErrorMessage error={password_error_msg.into()} />
                    </Show>

                    <form class="space-y-4" on:submit=on_submit_password>
                        <div>
                            <label class="block text-sm font-medium text-fg-muted">"現在のパスワード"</label>
                            <input type="password" required
                                class="mt-1 w-full border border-form-control-border bg-form-control-bg text-form-control-text rounded px-3 py-2"
                                on:input=move |ev| set_current_password.set(event_target_value(&ev))
                                prop:value=current_password
                            />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-fg-muted">"新しいパスワード"</label>
                            <input type="password" required
                                class="mt-1 w-full border border-form-control-border bg-form-control-bg text-form-control-text rounded px-3 py-2"
                                on:input=move |ev| set_new_password.set(event_target_value(&ev))
                                prop:value=new_password
                            />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-fg-muted">"新しいパスワード（確認）"</label>
                            <input type="password" required
                                class="mt-1 w-full border border-form-control-border bg-form-control-bg text-form-control-text rounded px-3 py-2"
                                on:input=move |ev| set_confirm_password.set(event_target_value(&ev))
                                prop:value=confirm_password
                            />
                        </div>
                        <div class="flex justify-end">
                            <button type="submit"
                                class="px-4 py-2 bg-action-primary-bg text-action-primary-text rounded hover:bg-action-primary-bg-hover disabled:opacity-50"
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
                <div class="bg-surface-elevated shadow rounded-lg p-6 space-y-4">
                    <h2 class="text-xl font-semibold text-fg border-b border-border pb-2">"本人対応申請"</h2>
                    <Show when=move || subject_success_msg.get().is_some() fallback=|| ()>
                        <SuccessMessage message={subject_success_msg.get().unwrap_or_default()} />
                    </Show>
                    <Show when=move || subject_error_msg.get().is_some() fallback=|| ()>
                        <ErrorMessage message={subject_error_msg.get().unwrap_or_default()} />
                    </Show>
                    <form class="space-y-3" on:submit=on_submit_subject>
                        <div>
                            <label class="block text-sm font-medium text-fg-muted">"申請種別"</label>
                            <select
                                class="mt-1 w-full border border-form-control-border bg-form-control-bg text-form-control-text rounded px-3 py-2"
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
                            <label class="block text-sm font-medium text-fg-muted">"詳細"</label>
                            <textarea
                                class="mt-1 w-full border border-form-control-border bg-form-control-bg text-form-control-text rounded px-3 py-2"
                                rows="3"
                                prop:value={move || subject_details.get()}
                                on:input=move |ev| subject_details.set(event_target_value(&ev))
                            ></textarea>
                        </div>
                        <div class="flex justify-end">
                            <button
                                type="submit"
                                class="px-4 py-2 bg-action-primary-bg text-action-primary-text rounded disabled:opacity-50"
                                disabled=move || subject_loading.get()
                            >
                                {move || if subject_loading.get() { "送信中..." } else { "申請する" }}
                            </button>
                        </div>
                    </form>
                    <div>
                        <h3 class="text-sm font-medium text-fg-muted mb-2">{"申請履歴"}</h3>
                        <Show when=move || subject_requests_error.get().is_some()>
                            <ErrorMessage message={subject_requests_error.get().unwrap_or_default()} />
                        </Show>
                        <div class="overflow-x-auto">
                            <table class="min-w-full divide-y divide-border">
                                <thead class="bg-surface-muted">
                                    <tr>
                                        <th class="px-4 py-2 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">{"種別"}</th>
                                        <th class="px-4 py-2 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">{"ステータス"}</th>
                                        <th class="px-4 py-2 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">{"申請日"}</th>
                                        <th class="px-4 py-2 text-right text-xs font-medium text-fg-muted uppercase tracking-wider">{"操作"}</th>
                                    </tr>
                                </thead>
                                <tbody class="bg-surface-elevated divide-y divide-border">
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
                                                        <td class="px-4 py-2 whitespace-nowrap text-sm text-fg">{type_label}</td>
                                                        <td class="px-4 py-2 whitespace-nowrap text-sm text-fg-muted">{status_label}</td>
                                                        <td class="px-4 py-2 whitespace-nowrap text-sm text-fg-muted">{created_label}</td>
                                                        <td class="px-4 py-2 whitespace-nowrap text-right text-sm">
                                                            <button
                                                                class="text-action-danger-bg hover:text-action-danger-bg-hover disabled:opacity-50"
                                                                disabled={move || {
                                                                    is_subject_cancel_disabled(
                                                                        cancel_loading.get(),
                                                                        can_cancel,
                                                                    )
                                                                }}
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
    use super::{
        map_change_password_error, normalize_subject_details, parse_subject_request_type,
        validate_password_submission,
    };
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

    #[wasm_bindgen_test]
    fn validate_password_submission_checks_constraints() {
        assert!(validate_password_submission("short", "short").is_err());
        assert!(validate_password_submission("12345678", "different").is_err());
        assert!(validate_password_submission("12345678", "12345678").is_ok());
    }

    #[wasm_bindgen_test]
    fn parse_subject_request_type_maps_values() {
        assert!(matches!(
            parse_subject_request_type("access"),
            Ok(crate::api::DataSubjectRequestType::Access)
        ));
        assert!(parse_subject_request_type("unknown").is_err());
    }

    #[wasm_bindgen_test]
    fn normalize_subject_details_trims_or_returns_none() {
        assert_eq!(
            normalize_subject_details("  memo  "),
            Some("memo".to_string())
        );
        assert_eq!(normalize_subject_details("   "), None);
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::api::test_support::mock::*;
    use crate::api::ApiClient;
    use crate::test_support::ssr::with_local_runtime_async;
    use serde_json::json;

    fn mock_server() -> MockServer {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/auth/mfa");
            then.status(200).json_body(json!({
                "enabled": false,
                "pending": false
            }));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/subject-requests/me");
            then.status(200).json_body(json!([]));
        });
        server
    }

    #[test]
    fn settings_page_renders_sections() {
        with_local_runtime_async(|| async {
            let runtime = leptos::create_runtime();
            let server = mock_server();
            provide_context(ApiClient::new_with_base_url(&server.url("/api")));

            leptos_reactive::suppress_resource_load(true);
            let html = view! { <SettingsPage /> }
                .into_view()
                .render_to_string()
                .to_string();
            leptos_reactive::suppress_resource_load(false);

            assert!(html.contains("パスワード変更"));
            assert!(html.contains("本人対応申請"));
            assert!(html.contains("MFA 設定"));

            runtime.dispose();
        });
    }

    #[test]
    fn helper_functions_cover_subject_and_password_validation() {
        assert!(validate_password_submission("short", "short").is_err());
        assert!(validate_password_submission("long-enough", "mismatch").is_err());
        assert!(validate_password_submission("long-enough", "long-enough").is_ok());

        assert!(matches!(
            parse_subject_request_type("rectify"),
            Ok(DataSubjectRequestType::Rectify)
        ));
        assert!(parse_subject_request_type("invalid").is_err());

        assert_eq!(
            normalize_subject_details("  test details  "),
            Some("test details".to_string())
        );
        assert_eq!(normalize_subject_details("   "), None);
    }

    #[test]
    fn helper_functions_cover_labels_and_datetime_format() {
        assert_eq!(
            subject_request_type_label(&DataSubjectRequestType::Access),
            "開示"
        );
        assert_eq!(
            subject_request_type_label(&DataSubjectRequestType::Rectify),
            "訂正"
        );
        assert_eq!(
            subject_request_type_label(&DataSubjectRequestType::Delete),
            "削除"
        );
        assert_eq!(
            subject_request_type_label(&DataSubjectRequestType::Stop),
            "停止"
        );

        assert_eq!(subject_request_status_label("pending"), "承認待ち");
        assert_eq!(subject_request_status_label("approved"), "承認済み");
        assert_eq!(subject_request_status_label("rejected"), "却下");
        assert_eq!(subject_request_status_label("cancelled"), "取消");
        assert_eq!(subject_request_status_label("custom"), "custom");

        let dt = DateTime::parse_from_rfc3339("2026-01-16T12:34:56Z")
            .expect("valid datetime")
            .with_timezone(&Utc);
        assert_eq!(format_subject_datetime(dt), "2026-01-16 12:34");
    }

    #[test]
    fn helper_functions_cover_password_error_mapping() {
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
        assert_eq!(
            map_change_password_error(&ApiError::unknown("other")).error,
            "パスワード変更に失敗しました。時間をおいて再度お試しください。"
        );
    }

    #[test]
    fn helper_password_submit_preparation_handles_pending_and_validation() {
        assert!(prepare_password_change_submission(
            true,
            "current".to_string(),
            "new-password".to_string(),
            "new-password".to_string(),
        )
        .expect("pending should be accepted")
        .is_none());

        assert!(prepare_password_change_submission(
            false,
            "current".to_string(),
            "short".to_string(),
            "short".to_string(),
        )
        .is_err());

        let payload = prepare_password_change_submission(
            false,
            "current".to_string(),
            "new-password".to_string(),
            "new-password".to_string(),
        )
        .expect("valid password payload")
        .expect("dispatch payload");
        assert_eq!(payload.0, "current");
        assert_eq!(payload.1, "new-password");
    }

    #[test]
    fn helper_mfa_activation_and_subject_submission_cover_branches() {
        assert!(prepare_mfa_activation_submission(true, "123456")
            .expect("pending should be accepted")
            .is_none());
        assert!(prepare_mfa_activation_submission(false, "123").is_err());
        assert_eq!(
            prepare_mfa_activation_submission(false, " 654321 ")
                .expect("valid mfa code")
                .expect("dispatch payload"),
            "654321"
        );

        assert!(prepare_subject_request_submission(true, "access", "memo")
            .expect("pending should be accepted")
            .is_none());
        assert!(prepare_subject_request_submission(false, "invalid", "memo").is_err());

        let payload = prepare_subject_request_submission(false, "delete", "  details ")
            .expect("valid subject request")
            .expect("dispatch payload");
        assert_eq!(payload.request_type, DataSubjectRequestType::Delete);
        assert_eq!(payload.details.as_deref(), Some("details"));

        let payload_blank_detail = prepare_subject_request_submission(false, "stop", "   ")
            .expect("valid subject request")
            .expect("dispatch payload");
        assert_eq!(
            payload_blank_detail.request_type,
            DataSubjectRequestType::Stop
        );
        assert_eq!(payload_blank_detail.details, None);
    }

    #[test]
    fn helper_feedback_mapping_covers_success_and_error() {
        let (ok_msg, ok_err) = password_change_feedback(Ok(()));
        assert_eq!(ok_msg.as_deref(), Some("パスワードを変更しました。"));
        assert!(ok_err.is_none());

        let (fail_msg, fail_err) =
            password_change_feedback(Err(ApiError::unknown("Current password is incorrect")));
        assert!(fail_msg.is_none());
        assert_eq!(
            fail_err.expect("mapped error").error,
            "現在のパスワードが正しくありません。"
        );

        let (create_ok_msg, create_ok_err) = subject_create_feedback(Ok(()));
        assert_eq!(
            create_ok_msg.as_deref(),
            Some("本人対応申請を送信しました。")
        );
        assert!(create_ok_err.is_none());

        let (create_fail_msg, create_fail_err) =
            subject_create_feedback::<()>(Err(ApiError::unknown("boom")));
        assert!(create_fail_msg.is_none());
        assert_eq!(create_fail_err.as_deref(), Some("boom"));

        let (cancel_ok_msg, cancel_ok_err) = subject_cancel_feedback(Ok(()));
        assert_eq!(
            cancel_ok_msg.as_deref(),
            Some("本人対応申請を取消しました。")
        );
        assert!(cancel_ok_err.is_none());

        let (cancel_fail_msg, cancel_fail_err) =
            subject_cancel_feedback(Err(ApiError::unknown("cancel failed")));
        assert!(cancel_fail_msg.is_none());
        assert_eq!(cancel_fail_err.as_deref(), Some("cancel failed"));
    }

    #[test]
    fn helper_subject_request_type_parsing_covers_all_known_values() {
        assert!(matches!(
            parse_subject_request_type("access"),
            Ok(DataSubjectRequestType::Access)
        ));
        assert!(matches!(
            parse_subject_request_type("rectify"),
            Ok(DataSubjectRequestType::Rectify)
        ));
        assert!(matches!(
            parse_subject_request_type("delete"),
            Ok(DataSubjectRequestType::Delete)
        ));
        assert!(matches!(
            parse_subject_request_type("stop"),
            Ok(DataSubjectRequestType::Stop)
        ));
    }

    #[test]
    fn helper_subject_cancel_disable_logic_covers_branches() {
        assert!(is_subject_cancel_disabled(true, true));
        assert!(is_subject_cancel_disabled(false, false));
        assert!(!is_subject_cancel_disabled(false, true));
    }
}
