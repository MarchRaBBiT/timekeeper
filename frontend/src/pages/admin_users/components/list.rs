use crate::{
    api::{ApiError, UserResponse},
    components::{error::InlineErrorMessage, layout::LoadingSpinner},
};
use leptos::*;

#[component]
pub fn UserList(
    users_resource: Resource<(bool, u32), Result<Vec<UserResponse>, ApiError>>,
    loading: Signal<bool>,
    on_select: Callback<UserResponse>,
) -> impl IntoView {
    let users = Signal::derive(move || {
        users_resource
            .get()
            .and_then(|result| result.ok())
            .unwrap_or_default()
    });
    let fetch_error = Signal::derive(move || users_resource.get().and_then(|result| result.err()));

    view! {
        <div class="bg-white dark:bg-gray-800 shadow rounded-lg p-6 space-y-4">
            <div class="flex flex-col gap-1 lg:flex-row lg:items-center lg:justify-between">
                <div>
                    <h3 class="text-lg font-medium text-gray-900 dark:text-gray-100">{"ユーザー一覧"}</h3>
                    <p class="text-sm text-gray-600 dark:text-gray-400">{"行をクリックすると詳細ドロワーが開きます。"}</p>
                </div>
            </div>

            <Show when=move || fetch_error.get().is_some()>
                <InlineErrorMessage error={fetch_error.into()} />
            </Show>
            <Show when=move || loading.get()>
                <LoadingSpinner />
            </Show>
            <Show when=move || !loading.get() && users.get().is_empty() && fetch_error.get().is_none()>
                <p class="text-sm text-gray-500">
                    {"登録済みのユーザーが見つかりません。新しいユーザーを招待してください。"}
                </p>
            </Show>
            <Show when=move || !users.get().is_empty()>
                <>
                    <div class="space-y-3 lg:hidden">
                        <For
                            each=move || users.get()
                            key=|user| user.id.clone()
                            children=move |user: UserResponse| {
                                let row_user = user.clone();
                                let click_handler = {
                                    let selected = row_user.clone();
                                    move |_| on_select.call(selected.clone())
                                };
                                view! {
                                    <button
                                        class="w-full text-left border border-gray-200 dark:border-gray-700 rounded-lg p-4 shadow-sm hover:bg-gray-50 dark:hover:bg-gray-700"
                                        on:click=click_handler
                                        type="button"
                                    >
                                        <div class="flex items-start justify-between gap-3">
                                            <div>
                                                <p class="text-sm text-gray-500 dark:text-gray-400">{"ユーザー名"}</p>
                                                <p class="text-base font-semibold text-gray-900 dark:text-gray-100">
                                                    {row_user.username}
                                                </p>
                                            </div>
                                            <div class="text-right">
                                                <p class="text-sm text-gray-500 dark:text-gray-400">{"権限"}</p>
                                                <p class="text-sm font-medium text-gray-900 dark:text-gray-100">
                                                    {row_user.role}
                                                </p>
                                            </div>
                                        </div>
                                        <div class="mt-3 grid grid-cols-2 gap-3 text-sm">
                                            <div>
                                                <p class="text-gray-500 dark:text-gray-400">{"氏名"}</p>
                                                <p class="text-gray-900 dark:text-gray-100">{row_user.full_name}</p>
                                            </div>
                                            <div>
                                                <p class="text-gray-500 dark:text-gray-400">{"システム管理者"}</p>
                                                <p class="text-gray-900 dark:text-gray-100">
                                                    {if row_user.is_system_admin { "Yes" } else { "No" }}
                                                </p>
                                            </div>
                                            <div>
                                                <p class="text-gray-500 dark:text-gray-400">{"MFA"}</p>
                                                <p class="text-gray-900 dark:text-gray-100">
                                                    {if row_user.mfa_enabled { "Enabled" } else { "Disabled" }}
                                                </p>
                                            </div>
                                        </div>
                                    </button>
                                }
                            }
                        />
                    </div>
                    <div class="hidden lg:block overflow-x-auto">
                        <table class="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
                            <thead class="bg-gray-50 dark:bg-gray-700">
                                <tr>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-200 uppercase tracking-wider">
                                        {"ユーザー名"}
                                    </th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-200 uppercase tracking-wider">
                                        {"氏名"}
                                    </th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-200 uppercase tracking-wider">
                                        {"権限"}
                                    </th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-200 uppercase tracking-wider">
                                        {"システム管理者"}
                                    </th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-200 uppercase tracking-wider">
                                        {"MFA"}
                                    </th>
                                </tr>
                            </thead>
                            <tbody class="bg-white dark:bg-gray-800 divide-y divide-gray-200 dark:divide-gray-700">
                                <For
                                    each=move || users.get()
                                    key=|user| user.id.clone()
                                    children=move |user: UserResponse| {
                                        let row_user = user.clone();
                                        let click_handler = {
                                            let selected = row_user.clone();
                                            move |_| on_select.call(selected.clone())
                                        };
                                        view! {
                                            <tr
                                                class="hover:bg-gray-50 dark:hover:bg-gray-700 cursor-pointer"
                                                on:click=click_handler
                                            >
                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900 dark:text-gray-100">
                                                    {row_user.username}
                                                </td>
                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900 dark:text-gray-100">
                                                    {row_user.full_name}
                                                </td>
                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900 dark:text-gray-100">
                                                    {row_user.role}
                                                </td>
                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900 dark:text-gray-100">
                                                    {if row_user.is_system_admin { "Yes" } else { "No" }}
                                                </td>
                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900 dark:text-gray-100">
                                                    {if row_user.mfa_enabled { "Enabled" } else { "Disabled" }}
                                                </td>
                                            </tr>
                                        }
                                    }
                                />
                            </tbody>
                        </table>
                    </div>
                </>
            </Show>
        </div>
    }
}
