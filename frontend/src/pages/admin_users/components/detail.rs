use crate::{
    api::{ApiError, UserResponse},
    components::{error::InlineErrorMessage, layout::SuccessMessage},
    pages::admin_users::utils::MessageState,
};
use leptos::*;

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
                        let is_self = current_user_id.get().map(|id| id == user_id).unwrap_or(false);

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


                        let soft_delete_click = move |_| {
                            hard_delete_mode.set(false);
                            show_delete_confirm.set(true);
                        };

                        let hard_delete_click = move |_| {
                            hard_delete_mode.set(true);
                            show_delete_confirm.set(true);
                        };

                        // Use selected_user signal to get user_id at runtime to avoid FnOnce issue
                        let confirm_delete = move |_| {
                            if let Some(current) = selected_user.get_untracked() {
                                messages.clear();
                                delete_user_action.dispatch((current.id.clone(), hard_delete_mode.get()));
                                show_delete_confirm.set(false);
                            }
                        };

                        let cancel_delete = move |_| {
                            show_delete_confirm.set(false);
                        };

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

                                        // Delete buttons (hidden for self)
                                        <Show when=move || !is_self>
                                            <Show
                                                when=move || !show_delete_confirm.get()
                                                fallback=move || {
                                                    view! {
                                                        <div class="border border-status-error-border rounded p-4 bg-status-error-bg text-status-error-text">
                                                            <p class="text-sm text-status-error-text mb-3">
                                                                {move || if hard_delete_mode.get() {
                                                                    "このユーザーと全ての関連データを完全に削除しますか？この操作は取り消せません。"
                                                                } else {
                                                                    "このユーザーを退職処理（アーカイブ）しますか？"
                                                                }}
                                                            </p>
                                                            <div class="flex gap-2">
                                                                <button
                                                                    class="flex-1 px-4 py-2 rounded bg-action-danger-bg text-action-danger-text disabled:opacity-50"
                                                                    disabled=move || delete_pending.get()
                                                                    on:click=confirm_delete
                                                                >
                                                                    {move || if delete_pending.get() { "処理中..." } else { "削除する" }}
                                                                </button>
                                                                <button
                                                                    class="flex-1 px-4 py-2 rounded bg-surface-muted text-fg"
                                                                    on:click=cancel_delete
                                                                >
                                                                    {"キャンセル"}
                                                                </button>
                                                            </div>
                                                        </div>
                                                    }
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
