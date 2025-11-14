use crate::api::{ApiClient, CreateUser, UserResponse};
use crate::components::layout::*;
use crate::state::auth::use_auth;
use leptos::*;
use wasm_bindgen::JsCast;

#[component]
pub fn AdminUsersPage() -> impl IntoView {
    let username = create_rw_signal(String::new());
    let full_name = create_rw_signal(String::new());
    let password = create_rw_signal(String::new());
    let role = create_rw_signal(String::from("employee"));
    let system_admin = create_rw_signal(false);
    let loading = create_rw_signal(false);
    let error = create_rw_signal(Option::<String>::None);
    let success = create_rw_signal(Option::<String>::None);

    let users = create_rw_signal(Vec::<UserResponse>::new());

    let (auth, _set_auth) = use_auth();
    let auth_for_guard = auth.clone();
    let is_system_admin = create_memo(move |_| {
        auth_for_guard
            .get()
            .user
            .as_ref()
            .map(|user| user.is_system_admin)
            .unwrap_or(false)
    });

    // Load current users list
    let load_users = {
        let users = users.clone();
        move || {
            leptos::spawn_local(async move {
                let api = ApiClient::new();
                match api.get_users().await {
                    Ok(list) => users.set(list),
                    Err(_) => {}
                }
            })
        }
    };
    {
        let load_users = load_users.clone();
        let is_system_admin = is_system_admin.clone();
        create_effect(move |_| {
            if !is_system_admin.get() {
                return;
            }
            load_users();
        });
    }

    let on_submit = {
        let is_system_admin = is_system_admin.clone();
        move |ev: leptos::ev::SubmitEvent| {
            ev.prevent_default();
            error.set(None);
            success.set(None);
            loading.set(true);
            if !is_system_admin.get_untracked() {
                error.set(Some("システム管理者のみ操作できます。".into()));
                loading.set(false);
                return;
            }
            let req = CreateUser {
                username: username.get(),
                password: password.get(),
                full_name: full_name.get(),
                role: role.get(),
                is_system_admin: system_admin.get(),
            };
            leptos::spawn_local(async move {
                let api = ApiClient::new();
                match api.create_user(req).await {
                    Ok(u) => {
                        success.set(Some(format!("ユーザー '{}' を作成しました", u.username)));
                        loading.set(false);
                        username.set(String::new());
                        password.set(String::new());
                        full_name.set(String::new());
                        role.set(String::from("employee"));
                        system_admin.set(false);
                        load_users();
                    }
                    Err(e) => {
                        error.set(Some(e));
                        loading.set(false);
                    }
                }
            });
        }
    };

    view! {
        <Layout>
            <Show
                when=move || is_system_admin.get()
                fallback=move || {
                    view! {
                        <div class="space-y-6">
                            <div class="bg-white shadow rounded-lg p-6">
                                <p class="text-sm text-gray-700">
                                    {"このページはシステム管理者のみ利用できます。"}
                                </p>
                            </div>
                        </div>
                    }
                }
            >
                <div class="space-y-6">
                <div class="bg-white shadow rounded-lg p-6">
                    <h2 class="text-lg font-medium text-gray-900 mb-4">{"ユーザー追加 (管理者専用)"}</h2>

                    <Show when=move || error.get().is_some()>
                        <ErrorMessage message={error.get().unwrap_or_default()} />
                    </Show>
                    <Show when=move || success.get().is_some()>
                        <SuccessMessage message={success.get().unwrap_or_default()} />
                    </Show>

                    <form class="grid grid-cols-1 md:grid-cols-2 gap-4" on:submit=on_submit>
                        <div>
                            <label class="block text-sm font-medium text-gray-700">{"ユーザー名"}</label>
                            <input class="mt-1 w-full border rounded px-2 py-1" placeholder="username" on:input=move |ev| username.set(event_target_value(&ev)) />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700">{"氏名"}</label>
                            <input class="mt-1 w-full border rounded px-2 py-1" placeholder="山田太郎" on:input=move |ev| full_name.set(event_target_value(&ev)) />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700">{"パスワード"}</label>
                            <input type="password" class="mt-1 w-full border rounded px-2 py-1" on:input=move |ev| password.set(event_target_value(&ev)) />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700">{"権限"}</label>
                            <select class="mt-1 w-full border rounded px-2 py-1" on:change=move |ev| role.set(event_target_value(&ev))>
                                <option value="employee" selected>{"employee"}</option>
                                <option value="admin">{"admin"}</option>
                            </select>
                        </div>
                        <div class="flex items-center space-x-2">
                            <input
                                type="checkbox"
                                class="h-4 w-4 text-blue-600 border-gray-300 rounded"
                                prop:checked=move || system_admin.get()
                                on:change=move |ev| {
                                    if let Some(target) = ev
                                        .target()
                                        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                                    {
                                        system_admin.set(target.checked());
                                    }
                                }
                            />
                            <label class="text-sm text-gray-700">{"システム管理者権限を付与"}</label>
                        </div>
                        <div class="md:col-span-2">
                            <button type="submit" disabled=loading.get() class="px-4 py-2 bg-blue-600 text-white rounded disabled:opacity-50">
                                {move || if loading.get() { "作成中..." } else { "ユーザーを作成" }}
                            </button>
                        </div>
                    </form>
                </div>

                <div class="bg-white shadow rounded-lg p-6">
                    <h3 class="text-lg font-medium text-gray-900 mb-4">{"ユーザー一覧"}</h3>
                    <div class="overflow-x-auto">
                        <table class="min-w-full divide-y divide-gray-200">
                            <thead>
                                <tr>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"ユーザー名"}</th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"氏名"}</th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"権限"}</th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"システム管理者"}</th>
                                </tr>
                            </thead>
                            <tbody class="bg-white divide-y divide-gray-200">
                                <For each=move || users.get() key=|u| u.id.clone() children=move |u| {
                                    view! {
                                        <tr>
                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{u.username.clone()}</td>
                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{u.full_name.clone()}</td>
                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{u.role.clone()}</td>
                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{if u.is_system_admin { "Yes" } else { "No" }}</td>
                                        </tr>
                                    }
                                } />
                            </tbody>
                        </table>
                    </div>
                </div>
                </div>
            </Show>
        </Layout>
    }
}
