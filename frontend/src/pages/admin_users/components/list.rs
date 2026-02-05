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
        <div class="bg-surface-elevated shadow rounded-lg p-6 space-y-4">
            <div class="flex flex-col gap-1 lg:flex-row lg:items-center lg:justify-between">
                <div>
                    <h3 class="text-lg font-medium text-fg">{"ユーザー一覧"}</h3>
                    <p class="text-sm text-fg-muted">{"行をクリックすると詳細ドロワーが開きます。"}</p>
                </div>
            </div>

            <Show when=move || fetch_error.get().is_some()>
                <InlineErrorMessage error={fetch_error.into()} />
            </Show>
            <Show when=move || loading.get()>
                <LoadingSpinner />
            </Show>
            <Show when=move || !loading.get() && users.get().is_empty() && fetch_error.get().is_none()>
                <p class="text-sm text-fg-muted">
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
                                        class="w-full text-left border border-border rounded-lg p-4 shadow-sm hover:bg-surface-muted"
                                        on:click=click_handler
                                        type="button"
                                    >
                                        <div class="flex items-start justify-between gap-3">
                                            <div>
                                                <p class="text-sm text-fg-muted">{"ユーザー名"}</p>
                                                <p class="text-base font-semibold text-fg">
                                                    {row_user.username}
                                                </p>
                                            </div>
                                            <div class="text-right">
                                                <p class="text-sm text-fg-muted">{"権限"}</p>
                                                <p class="text-sm font-medium text-fg">
                                                    {row_user.role}
                                                </p>
                                            </div>
                                        </div>
                                        <div class="mt-3 grid grid-cols-2 gap-3 text-sm">
                                            <div>
                                                <p class="text-fg-muted">{"氏名"}</p>
                                                <p class="text-fg">{row_user.full_name}</p>
                                            </div>
                                            <div>
                                                <p class="text-fg-muted">{"システム管理者"}</p>
                                                <p class="text-fg">
                                                    {if row_user.is_system_admin { "Yes" } else { "No" }}
                                                </p>
                                            </div>
                                            <div>
                                                <p class="text-fg-muted">{"MFA"}</p>
                                                <p class="text-fg">
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
                        <table class="min-w-full divide-y divide-border">
                            <thead class="bg-surface-muted">
                                <tr>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">
                                        {"ユーザー名"}
                                    </th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">
                                        {"氏名"}
                                    </th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">
                                        {"権限"}
                                    </th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">
                                        {"システム管理者"}
                                    </th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">
                                        {"MFA"}
                                    </th>
                                </tr>
                            </thead>
                            <tbody class="bg-surface-elevated divide-y divide-border">
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
                                                class="hover:bg-surface-muted cursor-pointer"
                                                on:click=click_handler
                                            >
                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-fg">
                                                    {row_user.username}
                                                </td>
                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-fg">
                                                    {row_user.full_name}
                                                </td>
                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-fg">
                                                    {row_user.role}
                                                </td>
                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-fg">
                                                    {if row_user.is_system_admin { "Yes" } else { "No" }}
                                                </td>
                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-fg">
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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::test_support::ssr::render_to_string;

    fn user() -> UserResponse {
        UserResponse {
            id: "u1".into(),
            username: "alice".into(),
            full_name: "Alice Example".into(),
            role: "admin".into(),
            is_system_admin: true,
            mfa_enabled: false,
        }
    }

    #[test]
    fn user_list_renders_empty_message() {
        let html = render_to_string(move || {
            let users = Resource::new(|| (true, 0u32), |_| async move { Ok(Vec::new()) });
            users.set(Ok(Vec::new()));
            let loading = Signal::derive(|| false);
            let on_select = Callback::new(|_| {});
            view! { <UserList users_resource=users loading=loading on_select=on_select /> }
        });
        assert!(html.contains("登録済みのユーザーが見つかりません"));
    }

    #[test]
    fn user_list_renders_rows() {
        let html = render_to_string(move || {
            let users = Resource::new(|| (true, 0u32), |_| async move { Ok(vec![user()]) });
            users.set(Ok(vec![user()]));
            let loading = Signal::derive(|| false);
            let on_select = Callback::new(|_| {});
            view! { <UserList users_resource=users loading=loading on_select=on_select /> }
        });
        assert!(html.contains("alice"));
        assert!(html.contains("Alice Example"));
    }
}
