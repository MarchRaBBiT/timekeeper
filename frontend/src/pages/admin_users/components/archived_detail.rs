use crate::{
    api::ArchivedUserResponse,
    components::layout::{ErrorMessage, SuccessMessage},
    pages::admin_users::utils::MessageState,
};
use leptos::*;

#[component]
pub fn ArchivedUserDetailDrawer(
    selected_archived_user: RwSignal<Option<ArchivedUserResponse>>,
    messages: MessageState,
    restore_action: Action<String, Result<(), String>>,
    delete_action: Action<String, Result<(), String>>,
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
                                <div class="absolute inset-0 bg-black/50" on:click=overlay_close></div>
                                <div class="relative w-full max-w-md bg-white shadow-xl h-full overflow-y-auto">
                                    <div class="flex items-center justify-between border-b px-6 py-4">
                                        <div>
                                            <h3 class="text-lg font-semibold text-gray-900">{user.full_name.clone()}</h3>
                                            <p class="text-sm text-gray-500">{format!("@{}", user.username)}</p>
                                        </div>
                                        <button class="text-gray-500 hover:text-gray-700" on:click=button_close>
                                            {"✕"}
                                        </button>
                                    </div>
                                    <div class="p-6 space-y-4">
                                        <div>
                                            <p class="text-sm text-gray-600">{"権限"}</p>
                                            <p class="text-base text-gray-900 font-medium">{user.role.clone()}</p>
                                        </div>
                                        <div>
                                            <p class="text-sm text-gray-600">{"システム管理者"}</p>
                                            <p class="text-base text-gray-900 font-medium">
                                                {if user.is_system_admin { "有効" } else { "無効" }}
                                            </p>
                                        </div>
                                        <div>
                                            <p class="text-sm text-gray-600">{"退職日"}</p>
                                            <p class="text-base text-gray-900 font-medium">
                                                {user.archived_at.split('T').next().unwrap_or(&user.archived_at).to_string()}
                                            </p>
                                        </div>
                                        <Show when=move || messages.error.get().is_some()>
                                            <ErrorMessage message={messages.error.get().unwrap_or_default()} />
                                        </Show>
                                        <Show when=move || messages.success.get().is_some()>
                                            <SuccessMessage message={messages.success.get().unwrap_or_default()} />
                                        </Show>

                                        // Action buttons
                                        <div class="border-t pt-4 mt-4 space-y-2">
                                            <button
                                                class="w-full px-4 py-2 rounded bg-green-600 text-white hover:bg-green-700 disabled:opacity-50"
                                                disabled=move || restore_pending.get() || delete_pending.get()
                                                on:click=restore_click
                                            >
                                                {move || if restore_pending.get() { "復職処理中..." } else { "復職させる" }}
                                            </button>

                                            <Show
                                                when=move || !show_delete_confirm.get()
                                                fallback=move || {
                                                    view! {
                                                        <div class="border border-red-200 rounded p-4 bg-red-50">
                                                            <p class="text-sm text-red-800 mb-3">
                                                                {"この退職ユーザーのデータを完全に削除しますか？この操作は取り消せません。"}
                                                            </p>
                                                            <div class="flex gap-2">
                                                                <button
                                                                    class="flex-1 px-4 py-2 rounded bg-red-600 text-white disabled:opacity-50"
                                                                    disabled=move || delete_pending.get()
                                                                    on:click=confirm_delete
                                                                >
                                                                    {move || if delete_pending.get() { "削除中..." } else { "完全削除する" }}
                                                                </button>
                                                                <button
                                                                    class="flex-1 px-4 py-2 rounded bg-gray-300 text-gray-700"
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
                                                    class="w-full px-4 py-2 rounded bg-red-600 text-white hover:bg-red-700 disabled:opacity-50"
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
