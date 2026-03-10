use crate::{
    api::{AdminSessionResponse, ApiError, UserResponse},
    components::{
        confirm_dialog::ConfirmDialog, error::InlineErrorMessage, layout::SuccessMessage,
    },
    pages::admin_users::utils::MessageState,
};
use leptos::*;

fn is_self_user(current_user_id: Option<&str>, target_user_id: &str) -> bool {
    current_user_id
        .map(|id| id == target_user_id)
        .unwrap_or(false)
}

fn delete_confirm_message(hard_delete_mode: bool) -> &'static str {
    if hard_delete_mode {
        "このユーザーと全ての関連データを完全に削除しますか？この操作は取り消せません。"
    } else {
        "このユーザーを退職処理（アーカイブ）しますか？"
    }
}

fn session_device_label(label: Option<&str>) -> &str {
    label.unwrap_or("不明なデバイス")
}

fn format_session_datetime(value: chrono::DateTime<chrono::Utc>) -> String {
    value.format("%Y-%m-%d %H:%M").to_string()
}

fn format_optional_session_datetime(value: Option<chrono::DateTime<chrono::Utc>>) -> String {
    value
        .map(format_session_datetime)
        .unwrap_or_else(|| "未記録".to_string())
}

#[component]
pub fn UserDetailDrawer(
    selected_user: RwSignal<Option<UserResponse>>,
    user_sessions_resource: Resource<
        (bool, Option<String>),
        Result<Vec<AdminSessionResponse>, ApiError>,
    >,
    messages: MessageState,
    reset_mfa_action: Action<String, Result<(), ApiError>>,
    unlock_user_action: Action<String, Result<(), ApiError>>,
    revoke_session_action: Action<String, Result<(), ApiError>>,
    delete_user_action: Action<(String, bool), Result<(), ApiError>>,
    /// Current user's ID to prevent self-deletion
    current_user_id: Signal<Option<String>>,
) -> impl IntoView {
    let pending = reset_mfa_action.pending();
    let unlock_pending = unlock_user_action.pending();
    let revoke_pending = revoke_session_action.pending();
    let delete_pending = delete_user_action.pending();

    // State for delete confirmation
    let show_delete_confirm = create_rw_signal(false);
    let hard_delete_mode = create_rw_signal(false);

    view! {
        <Show
            when=move || selected_user.get().is_some()
            fallback=|| view! {}.into_view()
        >
            {move || {
                selected_user
                    .get()
                    .map(|user| {
                        let user_id = user.id.clone();
                        let is_self =
                            is_self_user(current_user_id.get().as_deref(), user_id.as_str());

                        let overlay_close = {
                            move |_| {
                                messages.clear();
                                show_delete_confirm.set(false);
                                selected_user.set(None);
                            }
                        };
                        let button_close = {
                            move |_| {
                                messages.clear();
                                show_delete_confirm.set(false);
                                selected_user.set(None);
                            }
                        };
                        let reset_click = {
                            move |_| {
                                if let Some(current) = selected_user.get_untracked() {
                                    messages.clear();
                                    reset_mfa_action.dispatch(current.id.clone());
                                }
                            }
                        };
                        let unlock_click = {
                            move |_| {
                                if let Some(current) = selected_user.get_untracked() {
                                    messages.clear();
                                    unlock_user_action.dispatch(current.id.clone());
                                }
                            }
                        };


                        let soft_delete_click = move |_| {
                            hard_delete_mode.set(false);
                            show_delete_confirm.set(true);
                        };

                        let hard_delete_click = move |_| {
                            hard_delete_mode.set(true);
                            show_delete_confirm.set(true);
                        };

                        let confirm_delete = Callback::new(move |_| {
                            if let Some(current) = selected_user.get_untracked() {
                                messages.clear();
                                delete_user_action
                                    .dispatch((current.id.clone(), hard_delete_mode.get()));
                                show_delete_confirm.set(false);
                            }
                        });

                        let cancel_delete = Callback::new(move |_| {
                            show_delete_confirm.set(false);
                        });

                        view! {
                            <div class="fixed inset-0 z-50 flex justify-end">
                                <div class="absolute inset-0 bg-overlay-backdrop" on:click=overlay_close></div>
                                <div class="relative w-full max-w-md bg-surface-elevated shadow-xl h-full overflow-y-auto">
                                    <div class="flex items-center justify-between border-b border-border px-6 py-4">
                                        <div>
                                            <h3 class="text-lg font-semibold text-fg">{user.full_name}</h3>
                                            <p class="text-sm text-fg-muted">{format!("@{}", user.username)}</p>
                                        </div>
                                        <button class="text-fg-muted hover:text-fg" on:click=button_close>
                                            {"✕"}
                                        </button>
                                    </div>
                                    <div class="p-6 space-y-4">
                                        <div>
                                            <p class="text-sm text-fg-muted">{"権限"}</p>
                                            <p class="text-base text-fg font-medium">{user.role}</p>
                                        </div>
                                        <div>
                                            <p class="text-sm text-fg-muted">{"システム管理者"}</p>
                                            <p class="text-base text-fg font-medium">
                                                {if user.is_system_admin { "有効" } else { "無効" }}
                                            </p>
                                        </div>
                                        <div>
                                            <p class="text-sm text-fg-muted">{"MFA"}</p>
                                            <p class="text-base text-fg font-medium">
                                                {if user.mfa_enabled { "登録済み" } else { "未登録" }}
                                            </p>
                                        </div>
                                        <div>
                                            <p class="text-sm text-fg-muted">{"アカウント状態"}</p>
                                            <p class="text-base text-fg font-medium">
                                                {if user.is_locked { "ロック中" } else { "正常" }}
                                            </p>
                                        </div>
                                        <Show when=move || messages.error.get().is_some()>
                                            <InlineErrorMessage error={messages.error.into()} />
                                        </Show>
                                        <Show when=move || messages.success.get().is_some()>
                                            <SuccessMessage message={messages.success.get().unwrap_or_default()} />
                                        </Show>
                                        <button
                                            class="w-full px-4 py-2 rounded bg-action-primary-bg text-action-primary-text disabled:opacity-50"
                                            disabled=move || pending.get()
                                            on:click=reset_click
                                        >
                                            {move || if pending.get() { "MFA をリセット中..." } else { "MFA をリセット" }}
                                        </button>
                                        <Show when=move || user.is_locked>
                                            <button
                                                class="w-full px-4 py-2 rounded bg-status-warning-bg text-status-warning-text border border-status-warning-border disabled:opacity-50"
                                                disabled=move || unlock_pending.get()
                                                on:click=unlock_click
                                            >
                                                {move || if unlock_pending.get() { "ロック解除中..." } else { "ロックを解除" }}
                                            </button>
                                        </Show>
                                        <div class="border-t pt-4 mt-4 space-y-3">
                                            <div>
                                                <p class="text-sm text-fg-muted">{"アクティブセッション"}</p>
                                                <p class="text-xs text-fg-muted">{"対象ユーザーのログイン中デバイスを確認し、必要に応じて強制ログアウトできます。"}</p>
                                            </div>
                                            {move || {
                                                match user_sessions_resource.get() {
                                                    Some(Ok(sessions)) if sessions.is_empty() => view! {
                                                        <div class="rounded-lg border border-dashed border-border px-3 py-4 text-sm text-fg-muted">
                                                            {"アクティブなセッションはありません。"}
                                                        </div>
                                                    }.into_view(),
                                                    Some(Ok(sessions)) => view! {
                                                        <div class="space-y-2">
                                                            {sessions.into_iter().map(|session| {
                                                                let session_id = session.id.clone();
                                                                let is_current = session.is_current;
                                                                let device_label = session_device_label(session.device_label.as_deref()).to_string();
                                                                let created_at = format_session_datetime(session.created_at);
                                                                let last_seen_at = format_optional_session_datetime(session.last_seen_at);
                                                                let expires_at = format_session_datetime(session.expires_at);
                                                                view! {
                                                                    <div class="rounded-lg border border-border bg-surface-muted px-3 py-3 space-y-2">
                                                                        <div class="flex items-center justify-between gap-3">
                                                                            <div>
                                                                                <div class="text-sm font-medium text-fg">{device_label}</div>
                                                                                <div class="text-xs text-fg-muted">
                                                                                    {if is_current { "現在のセッション" } else { "他のデバイス" }}
                                                                                </div>
                                                                            </div>
                                                                            <button
                                                                                class="px-3 py-2 rounded bg-action-danger-bg text-action-danger-text hover:bg-action-danger-bg-hover disabled:opacity-50 disabled:cursor-not-allowed text-sm"
                                                                                disabled=move || is_current || revoke_pending.get()
                                                                                on:click=move |_| revoke_session_action.dispatch(session_id.clone())
                                                                            >
                                                                                {if is_current { "このセッションです" } else { "強制ログアウト" }}
                                                                            </button>
                                                                        </div>
                                                                        <dl class="grid grid-cols-1 gap-1 text-xs text-fg-muted">
                                                                            <div><dt class="inline font-medium">{"開始: "}</dt><dd class="inline text-fg">{created_at}</dd></div>
                                                                            <div><dt class="inline font-medium">{"最終利用: "}</dt><dd class="inline text-fg">{last_seen_at}</dd></div>
                                                                            <div><dt class="inline font-medium">{"有効期限: "}</dt><dd class="inline text-fg">{expires_at}</dd></div>
                                                                        </dl>
                                                                    </div>
                                                                }
                                                            }).collect_view()}
                                                        </div>
                                                    }.into_view(),
                                                    Some(Err(error)) => view! {
                                                        <div class="rounded-lg border border-status-error-border bg-status-error-bg px-3 py-3 text-sm text-status-error-text">
                                                            {error.to_string()}
                                                        </div>
                                                    }.into_view(),
                                                    None => view! {
                                                        <div class="rounded-lg border border-dashed border-border px-3 py-4 text-sm text-fg-muted">
                                                            {"セッションを読み込んでいます..."}
                                                        </div>
                                                    }.into_view(),
                                                }
                                            }}
                                        </div>

                                        // Delete buttons (hidden for self)
                                        <Show when=move || !is_self>
                                            <div class="border-t pt-4 mt-4 space-y-2">
                                                <p class="text-sm text-fg-muted">{"ユーザー削除"}</p>
                                                <button
                                                    class="w-full px-4 py-2 rounded bg-status-warning-text text-text-inverse disabled:opacity-50"
                                                    disabled=move || delete_pending.get()
                                                    on:click=soft_delete_click
                                                >
                                                    {"退職処理（アーカイブ）"}
                                                </button>
                                                <button
                                                    class="w-full px-4 py-2 rounded bg-action-danger-bg text-action-danger-text hover:bg-action-danger-bg-hover disabled:opacity-50"
                                                    disabled=move || delete_pending.get()
                                                    on:click=hard_delete_click
                                                >
                                                    {"完全削除"}
                                                </button>
                                            </div>
                                        </Show>
                                        <Show when=move || is_self>
                                            <p class="text-sm text-fg-muted italic">{"自分自身は削除できません。"}</p>
                                        </Show>
                                    </div>
                                    <ConfirmDialog
                                        is_open=Signal::derive(move || show_delete_confirm.get())
                                        title="ユーザー削除の確認"
                                        message=Signal::derive(move || delete_confirm_message(hard_delete_mode.get()).to_string())
                                        on_confirm=confirm_delete
                                        on_cancel=cancel_delete
                                        confirm_label=Signal::derive(move || if hard_delete_mode.get() { "完全削除する".to_string() } else { "退職処理する".to_string() })
                                        cancel_label="いいえ"
                                        confirm_disabled=Signal::derive(move || delete_pending.get())
                                        destructive=true
                                    />
                                </div>
                            </div>
                        }
                    })
                    .unwrap_or_else(|| view! { <div></div> })
            }}
        </Show>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::test_support::ssr::render_to_string;

    fn user() -> UserResponse {
        UserResponse {
            id: "u1".into(),
            username: "alice".into(),
            full_name: "Alice Example".into(),
            role: "admin".into(),
            is_system_admin: false,
            mfa_enabled: true,
            is_locked: false,
            locked_until: None,
            failed_login_attempts: 0,
        }
    }

    #[test]
    fn user_detail_drawer_renders() {
        let html = render_to_string(move || {
            let selected = create_rw_signal(Some(user()));
            let user_sessions = create_resource(
                move || (true, Some("u1".to_string())),
                move |_| async move { Ok::<_, ApiError>(Vec::<AdminSessionResponse>::new()) },
            );
            let messages = MessageState::default();
            let reset_action = create_action(|_: &String| async move { Ok(()) });
            let unlock_action = create_action(|_: &String| async move { Ok(()) });
            let revoke_session_action = create_action(|_: &String| async move { Ok(()) });
            let delete_action = create_action(|_: &(String, bool)| async move { Ok(()) });
            let current_user_id = Signal::derive(|| None::<String>);
            view! {
                <UserDetailDrawer
                    selected_user=selected
                    user_sessions_resource=user_sessions
                    messages=messages
                    reset_mfa_action=reset_action
                    unlock_user_action=unlock_action
                    revoke_session_action=revoke_session_action
                    delete_user_action=delete_action
                    current_user_id=current_user_id
                />
            }
        });
        assert!(html.contains("Alice Example"));
        assert!(html.contains("MFA"));
        assert!(html.contains("アクティブセッション"));
    }

    #[test]
    fn helper_is_self_and_delete_message_cover_branches() {
        assert!(is_self_user(Some("u1"), "u1"));
        assert!(!is_self_user(Some("u2"), "u1"));
        assert!(!is_self_user(None, "u1"));

        assert_eq!(
            delete_confirm_message(true),
            "このユーザーと全ての関連データを完全に削除しますか？この操作は取り消せません。"
        );
        assert_eq!(
            delete_confirm_message(false),
            "このユーザーを退職処理（アーカイブ）しますか？"
        );
        let dt = chrono::DateTime::parse_from_rfc3339("2026-03-10T10:00:00Z")
            .expect("valid datetime")
            .with_timezone(&chrono::Utc);
        assert_eq!(session_device_label(Some("Chrome")), "Chrome");
        assert_eq!(session_device_label(None), "不明なデバイス");
        assert_eq!(format_session_datetime(dt), "2026-03-10 10:00");
        assert_eq!(format_optional_session_datetime(None), "未記録");
    }

    #[test]
    fn user_detail_drawer_hides_delete_actions_for_self_user() {
        let html = render_to_string(move || {
            let selected = create_rw_signal(Some(user()));
            let user_sessions = create_resource(
                move || (true, Some("u1".to_string())),
                move |_| async move { Ok::<_, ApiError>(Vec::<AdminSessionResponse>::new()) },
            );
            let messages = MessageState::default();
            let reset_action = create_action(|_: &String| async move { Ok(()) });
            let unlock_action = create_action(|_: &String| async move { Ok(()) });
            let revoke_session_action = create_action(|_: &String| async move { Ok(()) });
            let delete_action = create_action(|_: &(String, bool)| async move { Ok(()) });
            let current_user_id = Signal::derive(|| Some("u1".to_string()));
            view! {
                <UserDetailDrawer
                    selected_user=selected
                    user_sessions_resource=user_sessions
                    messages=messages
                    reset_mfa_action=reset_action
                    unlock_user_action=unlock_action
                    revoke_session_action=revoke_session_action
                    delete_user_action=delete_action
                    current_user_id=current_user_id
                />
            }
        });
        assert!(html.contains("自分自身は削除できません。"));
        assert!(!html.contains("退職処理（アーカイブ）"));
        assert!(!html.contains("完全削除"));
    }
}
