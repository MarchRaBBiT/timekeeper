use crate::api::{ApiClient, CreateUser, UserResponse};
use crate::components::layout::*;
use leptos::*;

#[component]
pub fn AdminUsersPage() -> impl IntoView {
    let username = create_rw_signal(String::new());
    let full_name = create_rw_signal(String::new());
    let password = create_rw_signal(String::new());
    let role = create_rw_signal(String::from("employee"));
    let loading = create_rw_signal(false);
    let error = create_rw_signal(Option::<String>::None);
    let success = create_rw_signal(Option::<String>::None);

    let users = create_rw_signal(Vec::<UserResponse>::new());

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
    load_users();

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        error.set(None);
        success.set(None);
        loading.set(true);
        let req = CreateUser {
            username: username.get(),
            password: password.get(),
            full_name: full_name.get(),
            role: role.get(),
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
                    load_users();
                }
                Err(e) => {
                    error.set(Some(e));
                    loading.set(false);
                }
            }
        });
    };

    view! {
        <Layout>
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
                                </tr>
                            </thead>
                            <tbody class="bg-white divide-y divide-gray-200">
                                <For each=move || users.get() key=|u| u.id.clone() children=move |u| {
                                    view! {
                                        <tr>
                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{u.username.clone()}</td>
                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{u.full_name.clone()}</td>
                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{u.role.clone()}</td>
                                        </tr>
                                    }
                                } />
                            </tbody>
                        </table>
                    </div>
                </div>
            </div>
        </Layout>
    }
}
