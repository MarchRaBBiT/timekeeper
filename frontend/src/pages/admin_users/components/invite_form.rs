use crate::{
    api::{CreateUser, UserResponse},
    components::layout::{ErrorMessage, SuccessMessage},
    pages::admin_users::utils::{InviteFormState, MessageState},
};
use leptos::{ev, *};
use wasm_bindgen::JsCast;

#[component]
pub fn InviteForm(
    form_state: RwSignal<InviteFormState>,
    messages: RwSignal<MessageState>,
    invite_action: Action<CreateUser, Result<UserResponse, String>>,
    is_system_admin: Memo<bool>,
) -> impl IntoView {
    let pending = invite_action.pending();
    let on_submit = {
        move |ev: ev::SubmitEvent| {
            ev.prevent_default();
            messages.update(MessageState::clear);
            if !is_system_admin.get_untracked() {
                messages.update(|state| {
                    state.set_error("システム管理者のみ操作できます。");
                });
                return;
            }
            let current = form_state.get_untracked();
            if !current.is_valid() {
                messages.update(|state| {
                    state.set_error("すべての必須項目を入力してください。");
                });
                return;
            }
            invite_action.dispatch(current.to_request());
        }
    };

    view! {
        <div class="bg-white shadow rounded-lg p-6 space-y-4">
            <div>
                <h2 class="text-lg font-medium text-gray-900">{"ユーザー招待 (管理者専用)"}</h2>
                <p class="text-sm text-gray-600">{"ユーザー名・氏名・権限を入力し、必要に応じてシステム管理者権限を付与します。"}</p>
            </div>

            <Show when=move || messages.get().error.is_some()>
                <ErrorMessage message={messages.get().error.unwrap_or_default()} />
            </Show>
            <Show when=move || messages.get().success.is_some()>
                <SuccessMessage message={messages.get().success.unwrap_or_default()} />
            </Show>

            <form class="grid grid-cols-1 md:grid-cols-2 gap-4" on:submit=on_submit>
                <div>
                    <label class="block text-sm font-medium text-gray-700">{"ユーザー名"}</label>
                    <input
                        class="mt-1 w-full border rounded px-2 py-1"
                        placeholder="username"
                        prop:value=move || form_state.get().username
                        on:input=move |ev| {
                            let value = event_target_value(&ev);
                            form_state.update(|state| state.username = value);
                        }
                    />
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700">{"氏名"}</label>
                    <input
                        class="mt-1 w-full border rounded px-2 py-1"
                        placeholder="山田太郎"
                        prop:value=move || form_state.get().full_name
                        on:input=move |ev| {
                            let value = event_target_value(&ev);
                            form_state.update(|state| state.full_name = value);
                        }
                    />
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700">{"パスワード"}</label>
                    <input
                        type="password"
                        class="mt-1 w-full border rounded px-2 py-1"
                        prop:value=move || form_state.get().password
                        on:input=move |ev| {
                            let value = event_target_value(&ev);
                            form_state.update(|state| state.password = value);
                        }
                    />
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700">{"権限"}</label>
                    <select
                        class="mt-1 w-full border rounded px-2 py-1"
                        prop:value=move || form_state.get().role
                        on:change=move |ev| {
                            let value = event_target_value(&ev);
                            form_state.update(|state| state.role = value);
                        }
                    >
                        <option value="employee">{"employee"}</option>
                        <option value="admin">{"admin"}</option>
                    </select>
                </div>
                <div class="flex items-center space-x-2 md:col-span-2">
                    <input
                        type="checkbox"
                        class="h-4 w-4 text-blue-600 border-gray-300 rounded"
                        prop:checked=move || form_state.get().is_system_admin
                        on:change=move |ev| {
                            if let Some(target) =
                                ev.target().and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                            {
                                form_state.update(|state| state.is_system_admin = target.checked());
                            }
                        }
                    />
                    <span class="text-sm text-gray-700">{"システム管理者権限を付与"}</span>
                </div>
                <div class="md:col-span-2">
                    <button
                        type="submit"
                        disabled=move || pending.get()
                        class="px-4 py-2 bg-blue-600 text-white rounded disabled:opacity-50"
                    >
                        {move || if pending.get() { "作成中..." } else { "ユーザーを作成" }}
                    </button>
                </div>
            </form>
        </div>
    }
}
