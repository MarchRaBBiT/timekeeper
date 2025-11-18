use crate::api::{ApiClient, UserResponse};
use leptos::*;
use web_sys::HtmlSelectElement;

#[component]
pub fn AdminMfaResetSection(system_admin_allowed: Memo<bool>) -> impl IntoView {
    let mfa_users = create_rw_signal(Vec::<UserResponse>::new());
    let selected_mfa_user = create_rw_signal(String::new());
    let mfa_reset_message = create_rw_signal(None::<String>);

    {
        let system_admin_allowed = system_admin_allowed.clone();
        let mfa_users = mfa_users.clone();
        create_effect(move |_| {
            if !system_admin_allowed.get() {
                return;
            }
            let mfa_users = mfa_users.clone();
            spawn_local(async move {
                let api = ApiClient::new();
                if let Ok(users) = api.get_users().await {
                    mfa_users.set(users);
                }
            });
        });
    }

    let on_reset_mfa = {
        let selected_mfa_user = selected_mfa_user.clone();
        let mfa_reset_message = mfa_reset_message.clone();
        let mfa_users_signal = mfa_users.clone();
        move |_| {
            let target = selected_mfa_user.get();
            if target.is_empty() {
                mfa_reset_message.set(Some("ユーザーを選択してください".into()));
                return;
            }
            let msg = mfa_reset_message.clone();
            let user_id = target.clone();
            let display_name = mfa_users_signal
                .get()
                .into_iter()
                .find(|u| u.id == user_id)
                .map(|u| format!("{} ({})", u.full_name, u.username))
                .unwrap_or_else(|| user_id.clone());
            spawn_local(async move {
                let api = ApiClient::new();
                match api.admin_reset_mfa(&user_id).await {
                    Ok(_) => msg.set(Some(format!("{} のMFAをリセットしました。", display_name))),
                    Err(err) => msg.set(Some(format!("MFAリセットに失敗しました: {}", err))),
                }
            });
        }
    };

    view! {
        <Show when=move || system_admin_allowed.get()>
            <div class="bg-white shadow rounded-lg p-4 space-y-4">
                <h2 class="text-lg font-semibold text-gray-900">{"MFA リセット (システム管理者専用)"}</h2>
                <p class="text-sm text-gray-600">
                    {"対象となるユーザーの MFA 設定をリセットし、次回ログイン時に再登録を求めます。"}
                </p>
                <div>
                    <label for="mfa-reset-user" class="block text-sm font-medium text-gray-700">
                        {"対象ユーザー"}
                    </label>
                    <select
                        id="mfa-reset-user"
                        class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500"
                        on:change=move |ev| {
                            let target = event_target::<HtmlSelectElement>(&ev);
                            selected_mfa_user.set(target.value());
                        }
                    >
                        <option value="">{"ユーザーを選択してください"}</option>
                        {move || {
                            mfa_users
                                .get()
                                .into_iter()
                                .map(|user| {
                                    view! {
                                        <option value={user.id.clone()}>
                                            {format!("{} ({})", user.full_name, user.username)}
                                        </option>
                                    }
                                })
                                .collect_view()
                        }}
                    </select>
                </div>
                <div>
                    <button
                        on:click=on_reset_mfa
                        class="px-4 py-2 rounded-md bg-red-600 text-white hover:bg-red-700 disabled:opacity-50"
                    >
                        {"MFAをリセット"}
                    </button>
                </div>
                {move || {
                    mfa_reset_message
                        .get()
                        .map(|msg| view! { <p class="text-sm text-gray-700">{msg}</p> }.into_view())
                        .unwrap_or_else(|| view! {}.into_view())
                }}
            </div>
        </Show>
    }
}
