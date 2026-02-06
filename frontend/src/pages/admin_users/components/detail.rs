use crate::{
    api::{ApiError, UserResponse},
    components::{error::InlineErrorMessage, layout::SuccessMessage},
    pages::admin_users::utils::MessageState,
};
use leptos::{ev::MouseEvent, *};

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

fn system_admin_status_label(is_system_admin: bool) -> &'static str {
    if is_system_admin {
        "有効"
    } else {
        "無効"
    }
}

fn mfa_status_label(mfa_enabled: bool) -> &'static str {
    if mfa_enabled {
        "登録済み"
    } else {
        "未登録"
    }
}

fn reset_mfa_button_label(pending: bool) -> &'static str {
    if pending {
        "MFA をリセット中..."
    } else {
        "MFA をリセット"
    }
}

fn delete_confirm_button_label(delete_pending: bool) -> &'static str {
    if delete_pending {
        "処理中..."
    } else {
        "削除する"
    }
}

fn close_user_detail_drawer(
    selected_user: RwSignal<Option<UserResponse>>,
    messages: MessageState,
    show_delete_confirm: RwSignal<bool>,
) {
    messages.clear();
    show_delete_confirm.set(false);
    selected_user.set(None);
}

fn trigger_reset_mfa_if_selected<F>(
    selected_user: RwSignal<Option<UserResponse>>,
    messages: MessageState,
    dispatch: F,
) where
    F: FnOnce(String),
{
    if let Some(current) = selected_user.get_untracked() {
        messages.clear();
        dispatch(current.id);
    }
}

fn open_soft_delete_confirm(show_delete_confirm: RwSignal<bool>, hard_delete_mode: RwSignal<bool>) {
    hard_delete_mode.set(false);
    show_delete_confirm.set(true);
}

fn open_hard_delete_confirm(show_delete_confirm: RwSignal<bool>, hard_delete_mode: RwSignal<bool>) {
    hard_delete_mode.set(true);
    show_delete_confirm.set(true);
}

fn trigger_confirm_delete_if_selected<F>(
    selected_user: RwSignal<Option<UserResponse>>,
    messages: MessageState,
    show_delete_confirm: RwSignal<bool>,
    hard_delete_mode: RwSignal<bool>,
    dispatch: F,
) where
    F: FnOnce((String, bool)),
{
    if let Some(current) = selected_user.get_untracked() {
        messages.clear();
        dispatch((current.id, hard_delete_mode.get()));
        show_delete_confirm.set(false);
    }
}

fn cancel_delete_confirm(show_delete_confirm: RwSignal<bool>) {
    show_delete_confirm.set(false);
}

fn render_delete_confirm_panel(
    hard_delete_mode: RwSignal<bool>,
    delete_pending: Signal<bool>,
    on_confirm: Callback<MouseEvent>,
    on_cancel: Callback<MouseEvent>,
) -> impl IntoView {
    view! {
        <div class="border border-status-error-border rounded p-4 bg-status-error-bg text-status-error-text">
            <p class="text-sm text-status-error-text mb-3">
                {move || delete_confirm_message(hard_delete_mode.get())}
            </p>
            <div class="flex gap-2">
                <button
                    class="flex-1 px-4 py-2 rounded bg-action-danger-bg text-action-danger-text disabled:opacity-50"
                    disabled=move || delete_pending.get()
                    on:click=move |ev| on_confirm.call(ev)
                >
                    {move || delete_confirm_button_label(delete_pending.get())}
                </button>
                <button
                    class="flex-1 px-4 py-2 rounded bg-surface-muted text-fg"
                    on:click=move |ev| on_cancel.call(ev)
                >
                    {"キャンセル"}
                </button>
            </div>
        </div>
    }
}

#[component]
pub fn UserDetailDrawer(
    selected_user: RwSignal<Option<UserResponse>>,
    messages: MessageState,
    reset_mfa_action: Action<String, Result<(), ApiError>>,
    delete_user_action: Action<(String, bool), Result<(), ApiError>>,
    /// Current user's ID to prevent self-deletion
    current_user_id: Signal<Option<String>>,
) -> impl IntoView {
    let pending = reset_mfa_action.pending();
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

                        let overlay_close = move |_| {
                            close_user_detail_drawer(selected_user, messages, show_delete_confirm);
                        };
                        let button_close = move |_| {
                            close_user_detail_drawer(selected_user, messages, show_delete_confirm);
                        };
                        let reset_click = move |_| {
                            trigger_reset_mfa_if_selected(selected_user, messages, |id| {
                                reset_mfa_action.dispatch(id);
                            });
                        };

                        let soft_delete_click = move |_| {
                            open_soft_delete_confirm(show_delete_confirm, hard_delete_mode);
                        };

                        let hard_delete_click = move |_| {
                            open_hard_delete_confirm(show_delete_confirm, hard_delete_mode);
                        };

                        let confirm_delete = Callback::new(move |_| {
                            trigger_confirm_delete_if_selected(
                                selected_user,
                                messages,
                                show_delete_confirm,
                                hard_delete_mode,
                                |payload| delete_user_action.dispatch(payload),
                            );
                        });

                        let cancel_delete =
                            Callback::new(move |_| cancel_delete_confirm(show_delete_confirm));

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
                                                {system_admin_status_label(user.is_system_admin)}
                                            </p>
                                        </div>
                                        <div>
                                            <p class="text-sm text-fg-muted">{"MFA"}</p>
                                            <p class="text-base text-fg font-medium">
                                                {mfa_status_label(user.mfa_enabled)}
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
                                            {move || reset_mfa_button_label(pending.get())}
                                        </button>

                                        // Delete buttons (hidden for self)
                                        <Show when=move || !is_self>
                                            <Show
                                                when=move || !show_delete_confirm.get()
                                                fallback=move || {
                                                    render_delete_confirm_panel(
                                                        hard_delete_mode,
                                                        delete_pending.into(),
                                                        confirm_delete,
                                                        cancel_delete,
                                                    )
                                                }
                                            >
                                                <div class="border-t pt-4 mt-4 space-y-2">
                                                    <p class="text-sm text-fg-muted">{"ユーザー削除"}</p>
                                                    <button
                                                        class="w-full px-4 py-2 rounded bg-status-warning-text text-text-inverse disabled:opacity-50"
                                                        disabled=move || delete_pending.get()
                                                        on:click=soft_delete_click.clone()
                                                    >
                                                        {"退職処理（アーカイブ）"}
                                                    </button>
                                                    <button
                                                        class="w-full px-4 py-2 rounded bg-action-danger-bg text-action-danger-text hover:bg-action-danger-bg-hover disabled:opacity-50"
                                                        disabled=move || delete_pending.get()
                                                        on:click=hard_delete_click.clone()
                                                    >
                                                        {"完全削除"}
                                                    </button>
                                                </div>
                                            </Show>
                                        </Show>
                                        <Show when=move || is_self>
                                            <p class="text-sm text-fg-muted italic">{"自分自身は削除できません。"}</p>
                                        </Show>
                                    </div>
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
    use std::cell::RefCell;

    use crate::test_support::ssr::render_to_string;
    use crate::test_support::ssr::with_runtime;

    fn user() -> UserResponse {
        UserResponse {
            id: "u1".into(),
            username: "alice".into(),
            full_name: "Alice Example".into(),
            role: "admin".into(),
            is_system_admin: false,
            mfa_enabled: true,
        }
    }

    #[test]
    fn user_detail_drawer_renders() {
        let html = render_to_string(move || {
            let selected = create_rw_signal(Some(user()));
            let messages = MessageState::default();
            let reset_action = create_action(|_: &String| async move { Ok(()) });
            let delete_action = create_action(|_: &(String, bool)| async move { Ok(()) });
            let current_user_id = Signal::derive(|| None::<String>);
            view! {
                <UserDetailDrawer
                    selected_user=selected
                    messages=messages
                    reset_mfa_action=reset_action
                    delete_user_action=delete_action
                    current_user_id=current_user_id
                />
            }
        });
        assert!(html.contains("Alice Example"));
        assert!(html.contains("MFA"));
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
    }

    #[test]
    fn user_detail_drawer_hides_delete_actions_for_self_user() {
        let html = render_to_string(move || {
            let selected = create_rw_signal(Some(user()));
            let messages = MessageState::default();
            let reset_action = create_action(|_: &String| async move { Ok(()) });
            let delete_action = create_action(|_: &(String, bool)| async move { Ok(()) });
            let current_user_id = Signal::derive(|| Some("u1".to_string()));
            view! {
                <UserDetailDrawer
                    selected_user=selected
                    messages=messages
                    reset_mfa_action=reset_action
                    delete_user_action=delete_action
                    current_user_id=current_user_id
                />
            }
        });
        assert!(html.contains("自分自身は削除できません。"));
        assert!(!html.contains("退職処理（アーカイブ）"));
        assert!(!html.contains("完全削除"));
    }

    #[test]
    fn user_detail_drawer_renders_delete_actions_for_non_self_user() {
        let html = render_to_string(move || {
            let selected = create_rw_signal(Some(user()));
            let messages = MessageState::default();
            let reset_action = create_action(|_: &String| async move { Ok(()) });
            let delete_action = create_action(|_: &(String, bool)| async move { Ok(()) });
            let current_user_id = Signal::derive(|| Some("u2".to_string()));
            view! {
                <UserDetailDrawer
                    selected_user=selected
                    messages=messages
                    reset_mfa_action=reset_action
                    delete_user_action=delete_action
                    current_user_id=current_user_id
                />
            }
        });
        assert!(html.contains("退職処理（アーカイブ）"));
        assert!(html.contains("完全削除"));
    }

    #[test]
    fn user_detail_drawer_renders_nothing_when_no_user_selected() {
        let html = render_to_string(move || {
            let selected = create_rw_signal(None::<UserResponse>);
            let messages = MessageState::default();
            let reset_action = create_action(|_: &String| async move { Ok(()) });
            let delete_action = create_action(|_: &(String, bool)| async move { Ok(()) });
            let current_user_id = Signal::derive(|| None::<String>);
            view! {
                <UserDetailDrawer
                    selected_user=selected
                    messages=messages
                    reset_mfa_action=reset_action
                    delete_user_action=delete_action
                    current_user_id=current_user_id
                />
            }
        });
        assert!(!html.contains("Alice Example"));
        assert!(!html.contains("MFA"));
    }

    #[test]
    fn helper_status_and_button_labels_cover_branches() {
        assert_eq!(system_admin_status_label(true), "有効");
        assert_eq!(system_admin_status_label(false), "無効");
        assert_eq!(mfa_status_label(true), "登録済み");
        assert_eq!(mfa_status_label(false), "未登録");
        assert_eq!(reset_mfa_button_label(true), "MFA をリセット中...");
        assert_eq!(reset_mfa_button_label(false), "MFA をリセット");
        assert_eq!(delete_confirm_button_label(true), "処理中...");
        assert_eq!(delete_confirm_button_label(false), "削除する");
    }

    #[test]
    fn helper_drawer_state_and_dispatch_cover_paths() {
        with_runtime(|| {
            let selected_user = create_rw_signal(Some(user()));
            let show_delete_confirm = create_rw_signal(true);
            let messages = MessageState::default();

            messages.set_success("ok");
            close_user_detail_drawer(selected_user, messages, show_delete_confirm);
            assert!(selected_user.get().is_none());
            assert!(!show_delete_confirm.get());
            assert!(messages.success.get().is_none());
            assert!(messages.error.get().is_none());

            let selected_user = create_rw_signal(Some(user()));
            let show_delete_confirm = create_rw_signal(false);
            let hard_delete_mode = create_rw_signal(false);

            open_soft_delete_confirm(show_delete_confirm, hard_delete_mode);
            assert!(show_delete_confirm.get());
            assert!(!hard_delete_mode.get());

            open_hard_delete_confirm(show_delete_confirm, hard_delete_mode);
            assert!(show_delete_confirm.get());
            assert!(hard_delete_mode.get());

            cancel_delete_confirm(show_delete_confirm);
            assert!(!show_delete_confirm.get());

            let reset_dispatched = RefCell::new(None::<String>);
            trigger_reset_mfa_if_selected(selected_user, messages, |id| {
                *reset_dispatched.borrow_mut() = Some(id);
            });
            assert_eq!(reset_dispatched.borrow().as_deref(), Some("u1"));

            let selected_none = create_rw_signal(None::<UserResponse>);
            trigger_reset_mfa_if_selected(selected_none, messages, |_| {
                panic!("should not dispatch when no selected user")
            });

            let delete_dispatched = RefCell::new(None::<(String, bool)>);
            let show_delete_confirm = create_rw_signal(true);
            let hard_delete_mode = create_rw_signal(true);
            trigger_confirm_delete_if_selected(
                selected_user,
                messages,
                show_delete_confirm,
                hard_delete_mode,
                |payload| {
                    *delete_dispatched.borrow_mut() = Some(payload);
                },
            );
            assert_eq!(
                delete_dispatched.borrow().as_ref(),
                Some(&(String::from("u1"), true))
            );
            assert!(!show_delete_confirm.get());

            let show_delete_confirm = create_rw_signal(true);
            trigger_confirm_delete_if_selected(
                selected_none,
                messages,
                show_delete_confirm,
                hard_delete_mode,
                |_| panic!("should not dispatch without selected user"),
            );
            assert!(show_delete_confirm.get());
        });
    }

    #[test]
    fn helper_delete_confirm_panel_renders_mode_and_pending_labels() {
        let soft_html = render_to_string(move || {
            let hard_delete_mode = create_rw_signal(false);
            let delete_pending = Signal::derive(|| false);
            let on_confirm = Callback::new(|_| {});
            let on_cancel = Callback::new(|_| {});
            render_delete_confirm_panel(hard_delete_mode, delete_pending, on_confirm, on_cancel)
        });
        assert!(soft_html.contains("このユーザーを退職処理（アーカイブ）しますか？"));
        assert!(soft_html.contains("削除する"));

        let hard_html = render_to_string(move || {
            let hard_delete_mode = create_rw_signal(true);
            let delete_pending = Signal::derive(|| true);
            let on_confirm = Callback::new(|_| {});
            let on_cancel = Callback::new(|_| {});
            render_delete_confirm_panel(hard_delete_mode, delete_pending, on_confirm, on_cancel)
        });
        assert!(hard_html.contains(
            "このユーザーと全ての関連データを完全に削除しますか？この操作は取り消せません。"
        ));
        assert!(hard_html.contains("処理中..."));
        assert!(hard_html.contains("キャンセル"));
    }
}
