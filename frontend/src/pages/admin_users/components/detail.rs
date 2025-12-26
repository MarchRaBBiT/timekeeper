use crate::{
    api::UserResponse,
    components::layout::{ErrorMessage, SuccessMessage},
    pages::admin_users::utils::MessageState,
};
use leptos::*;

#[component]
pub fn UserDetailDrawer(
    selected_user: RwSignal<Option<UserResponse>>,
    messages: MessageState,
    reset_mfa_action: Action<String, Result<(), String>>,
) -> impl IntoView {
    let pending = reset_mfa_action.pending();
    // Effect moved to view model, or if we need local effect for success message?
    // The previous code had effect here. But actions are now in VM.
    // VM handles updating `messages` on action completion.
    // Wait, the component also defined an effect using `reset_mfa_action`.
    // If VM handles it, I don't need it here.
    // I should check `panel.rs` or VM.
    // VM has: `create_effect` for reset_mfa_action result.
    // So I can remove the effect from here!

    view! {
        <Show
            when=move || selected_user.get().is_some()
            fallback=|| view! {}.into_view()
        >
            {move || {
                selected_user
                    .get()
                    .map(|user| {
                        let overlay_close = {
                            move |_| {
                                messages.clear();
                                selected_user.set(None);
                            }
                        };
                        let button_close = {
                            move |_| {
                                messages.clear();
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
                                            <p class="text-sm text-gray-600">{"MFA"}</p>
                                            <p class="text-base text-gray-900 font-medium">
                                                {if user.mfa_enabled { "登録済み" } else { "未登録" }}
                                            </p>
                                        </div>
                                        <Show when=move || messages.error.get().is_some()>
                                            <ErrorMessage message={messages.error.get().unwrap_or_default()} />
                                        </Show>
                                        <Show when=move || messages.success.get().is_some()>
                                            <SuccessMessage message={messages.success.get().unwrap_or_default()} />
                                        </Show>
                                        <button
                                            class="w-full px-4 py-2 rounded bg-indigo-600 text-white disabled:opacity-50"
                                            disabled=move || pending.get()
                                            on:click=reset_click
                                        >
                                            {move || if pending.get() { "MFA をリセット中..." } else { "MFA をリセット" }}
                                        </button>
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
