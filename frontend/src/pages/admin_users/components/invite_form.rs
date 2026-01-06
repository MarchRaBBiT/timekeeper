use crate::{
    api::{ApiError, CreateUser, UserResponse},
    components::{error::InlineErrorMessage, layout::SuccessMessage},
    pages::admin_users::utils::{InviteFormState, MessageState},
};
use leptos::{ev, *};
use wasm_bindgen::JsCast;

#[component]
pub fn InviteForm(
    form_state: InviteFormState,
    messages: MessageState,
    invite_action: Action<CreateUser, Result<UserResponse, ApiError>>,
    is_system_admin: Memo<bool>,
) -> impl IntoView {
    let pending = invite_action.pending();
    let on_submit = {
        move |ev: ev::SubmitEvent| {
            ev.prevent_default();
            messages.clear();
            if !is_system_admin.get_untracked() {
                messages.set_error(ApiError::unknown("システム管理者のみ操作できます。"));
                return;
            }
            if !form_state.is_valid() {
                messages.set_error(ApiError::validation("すべての必須項目を入力してください。"));
                return;
            }
            invite_action.dispatch(form_state.to_request());
        }
    };

    view! {
        <div class="bg-white shadow rounded-lg p-6 space-y-4">
            <div>
                <h2 class="text-lg font-medium text-gray-900">{"ユーザー招待 (管理者専用)"}</h2>
                <p class="text-sm text-gray-600">{"ユーザー名・氏名・権限を入力し、必要に応じてシステム管理者権限を付与します。"}</p>
            </div>

            <Show when=move || messages.error.get().is_some()>
                <InlineErrorMessage error={messages.error.into()} />
            </Show>
            <Show when=move || messages.success.get().is_some()>
                <SuccessMessage message={messages.success.get().unwrap_or_default()} />
            </Show>

            <form class="grid grid-cols-1 lg:grid-cols-2 gap-4" on:submit=on_submit>
                <div>
                    <label class="block text-sm font-medium text-gray-700">{"ユーザー名"}</label>
                    <input
                        class="mt-1 w-full border rounded px-2 py-1"
                        placeholder="username"
                        prop:value=form_state.username
                        on:input=move |ev| form_state.username.set(event_target_value(&ev))
                    />
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700">{"氏名"}</label>
                    <input
                        class="mt-1 w-full border rounded px-2 py-1"
                        placeholder="山田太郎"
                        prop:value=form_state.full_name
                        on:input=move |ev| form_state.full_name.set(event_target_value(&ev))
                    />
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700">{"パスワード"}</label>
                    <input
                        type="password"
                        class="mt-1 w-full border rounded px-2 py-1"
                        prop:value=form_state.password
                        on:input=move |ev| form_state.password.set(event_target_value(&ev))
                    />
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700">{"権限"}</label>
                    <select
                        class="mt-1 w-full border rounded px-2 py-1"
                        prop:value=form_state.role
                        on:change=move |ev| form_state.role.set(event_target_value(&ev))
                    >
                        <option value="employee">{"employee"}</option>
                        <option value="admin">{"admin"}</option>
                    </select>
                </div>
                <div class="flex items-center space-x-2 lg:col-span-2">
                    <input
                        type="checkbox"
                        class="h-4 w-4 text-blue-600 border-gray-300 rounded"
                        prop:checked=form_state.is_system_admin
                        on:change=move |ev| {
                            if let Some(target) =
                                ev.target().and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                            {
                                form_state.is_system_admin.set(target.checked());
                            }
                        }
                    />
                    <span class="text-sm text-gray-700">{"システム管理者権限を付与"}</span>
                </div>
                <div class="lg:col-span-2">
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
