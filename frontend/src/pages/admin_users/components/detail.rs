use crate::{
    api::UserResponse,
    components::layout::{ErrorMessage, SuccessMessage},
    pages::admin_users::utils::MessageState,
};
use leptos::*;

#[component]
pub fn UserDetailDrawer(
    selected_user: RwSignal<Option<UserResponse>>,
    messages: RwSignal<MessageState>,
    reset_mfa_action: Action<String, Result<(), String>>,
) -> impl IntoView {
    let pending = reset_mfa_action.pending();
    {
        let messages = messages.clone();
        let reset_mfa_action = reset_mfa_action.clone();
        create_effect(move |_| {
            if let Some(result) = reset_mfa_action.value().get() {
                match result {
                    Ok(_) => {
                        messages.update(|state| {
                            state.set_success("MFA をリセットしました。");
                        });
                    }
                    Err(err) => {
                        messages.update(|state| {
                            state.set_error(err);
                        });
                    }
                }
            }
        });
    }

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
                            let selected_user = selected_user.clone();
                            let messages = messages.clone();
                            move |_| {
                                messages.update(MessageState::clear);
                                selected_user.set(None);
                            }
                        };
                        let button_close = {
                            let selected_user = selected_user.clone();
                            let messages = messages.clone();
                            move |_| {
                                messages.update(MessageState::clear);
                                selected_user.set(None);
                            }
                        };
                        let reset_click = {
                            let selected_user = selected_user.clone();
                            let messages = messages.clone();
                            move |_| {
                                if let Some(current) = selected_user.get_untracked() {
                                    messages.update(MessageState::clear);
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
                                        <Show when=move || messages.get().error.is_some()>
                                            <ErrorMessage message={messages.get().error.unwrap_or_default()} />
                                        </Show>
                                        <Show when=move || messages.get().success.is_some()>
                                            <SuccessMessage message={messages.get().success.unwrap_or_default()} />
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
