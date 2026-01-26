use crate::{
    api::{ApiError, ArchivedUserResponse},
    components::{error::InlineErrorMessage, layout::SuccessMessage},
    pages::admin_users::utils::MessageState,
};
use leptos::*;

#[component]
pub fn ArchivedUserDetailDrawer(
    selected_archived_user: RwSignal<Option<ArchivedUserResponse>>,
    messages: MessageState,
    restore_action: Action<String, Result<(), ApiError>>,
    delete_action: Action<String, Result<(), ApiError>>,
) -> impl IntoView {
    let restore_pending = restore_action.pending();
    let delete_pending = delete_action.pending();

    // State for delete confirmation
    let show_delete_confirm = create_rw_signal(false);

    view! {
        <Show
            when=move || selected_archived_user.get().is_some()
            fallback=|| view! {}.into_view()
        >
            {move || {
                selected_archived_user
                    .get()
                    .map(|user| {
                        let overlay_close = {
                            move |_| {
                                messages.clear();
                                show_delete_confirm.set(false);
                                selected_archived_user.set(None);
                            }
                        };
                        let button_close = {
                            move |_| {
                                messages.clear();
                                show_delete_confirm.set(false);
                                selected_archived_user.set(None);
                            }
                        };

                        let restore_click = {
                            move |_| {
                                if let Some(current) = selected_archived_user.get_untracked() {
                                    messages.clear();
                                    restore_action.dispatch(current.id.clone());
                                }
                            }
                        };

                        let delete_click = move |_| {
                            show_delete_confirm.set(true);
                        };

                        let confirm_delete = move |_| {
                            if let Some(current) = selected_archived_user.get_untracked() {
                                messages.clear();
                                delete_action.dispatch(current.id.clone());
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
                                            <h3 class="text-lg font-semibold text-fg">{user.full_name.clone()}</h3>
                                            <p class="text-sm text-fg-muted">{format!("@{}", user.username)}</p>
                                        </div>
                                        <button class="text-fg-muted hover:text-fg" on:click=button_close>
                                            {"✕"}
                                        </button>
                                    </div>
                                    <div class="p-6 space-y-4">
                                        <div>
                                            <p class="text-sm text-fg-muted">{"権限"}</p>
                                            <p class="text-base text-fg font-medium">{user.role.clone()}</p>
                                        </div>
                                        <div>
                                            <p class="text-sm text-fg-muted">{"システム管理者"}</p>
                                            <p class="text-base text-fg font-medium">
                                                {if user.is_system_admin { "有効" } else { "無効" }}
                                            </p>
                                        </div>
                                        <div>
                                            <p class="text-sm text-fg-muted">{"退職日"}</p>
                                            <p class="text-base text-fg font-medium">
                                                {user.archived_at.split('T').next().unwrap_or(&user.archived_at).to_string()}
                                            </p>
                                        </div>
                                        <Show when=move || messages.error.get().is_some()>
                                            <InlineErrorMessage error={messages.error.into()} />
                                        </Show>
                                        <Show when=move || messages.success.get().is_some()>
                                            <SuccessMessage message={messages.success.get().unwrap_or_default()} />
                                        </Show>

                                        // Action buttons
                                        <div class="border-t pt-4 mt-4 space-y-2">
                                            <button
                                                class="w-full px-4 py-2 rounded bg-action-primary-bg text-action-primary-text hover:bg-action-primary-bg-hover disabled:opacity-50"
                                                disabled=move || restore_pending.get() || delete_pending.get()
                                                on:click=restore_click
                                            >
                                                {move || if restore_pending.get() { "復職処理中..." } else { "復職させる" }}
                                            </button>

                                            <Show
                                                when=move || !show_delete_confirm.get()
                                                fallback=move || {
                                                    view! {
                                                        <div class="border border-status-error-border rounded p-4 bg-status-error-bg text-status-error-text">
                                                            <p class="text-sm text-status-error-text mb-3">
                                                                {"この退職ユーザーのデータを完全に削除しますか？この操作は取り消せません。"}
                                                            </p>
                                                            <div class="flex gap-2">
                                                                <button
                                                                    class="flex-1 px-4 py-2 rounded bg-action-danger-bg text-action-danger-text disabled:opacity-50"
                                                                    disabled=move || delete_pending.get()
                                                                    on:click=confirm_delete
                                                                >
                                                                    {move || if delete_pending.get() { "削除中..." } else { "完全削除する" }}
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
                                                <button
                                                    class="w-full px-4 py-2 rounded bg-action-danger-bg text-action-danger-text hover:bg-action-danger-bg-hover disabled:opacity-50"
                                                    disabled=move || restore_pending.get() || delete_pending.get()
                                                    on:click=delete_click
                                                >
                                                    {"完全削除"}
                                                </button>
                                            </Show>
                                        </div>
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
