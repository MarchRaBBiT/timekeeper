use crate::{
    api::{ApiError, PiiProtectedResponse, UserResponse},
    components::{error::InlineErrorMessage, layout::LoadingSpinner},
    pages::admin_users::utils::localized_role_label,
};
use leptos::*;

type UsersResource =
    Resource<(bool, u32), Result<PiiProtectedResponse<Vec<UserResponse>>, ApiError>>;

#[component]
pub fn UserList(
    users_resource: UsersResource,
    loading: Signal<bool>,
    on_select: Callback<UserResponse>,
) -> impl IntoView {
    let users = Signal::derive(move || {
        users_resource
            .get()
            .and_then(|result| result.ok().map(|payload| payload.data))
            .unwrap_or_default()
    });
    let fetch_error = Signal::derive(move || users_resource.get().and_then(|result| result.err()));

    view! {
        <div class="bg-surface-elevated shadow rounded-lg p-6 space-y-4">
            <div class="flex flex-col gap-1 lg:flex-row lg:items-center lg:justify-between">
                <div>
                    <h3 class="text-lg font-medium text-fg">
                        {rust_i18n::t!("pages.admin_users.list.title")}
                    </h3>
                    <p class="text-sm text-fg-muted">
                        {rust_i18n::t!("pages.admin_users.list.description")}
                    </p>
                </div>
            </div>

            <Show when=move || fetch_error.get().is_some()>
                <InlineErrorMessage error={fetch_error} />
            </Show>
            <Show when=move || loading.get()>
                <LoadingSpinner />
            </Show>
            <Show when=move || !loading.get() && users.get().is_empty() && fetch_error.get().is_none()>
                <p class="text-sm text-fg-muted">
                    {rust_i18n::t!("pages.admin_users.list.empty")}
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
                                                <p class="text-sm text-fg-muted">
                                                    {rust_i18n::t!(
                                                        "pages.admin_users.list.columns.username"
                                                    )}
                                                </p>
                                                <p class="text-base font-semibold text-fg">
                                                    {row_user.username}
                                                </p>
                                            </div>
                                            <div class="text-right">
                                                <p class="text-sm text-fg-muted">
                                                    {rust_i18n::t!(
                                                        "pages.admin_users.list.columns.role"
                                                    )}
                                                </p>
                                                <p class="text-sm font-medium text-fg">
                                                    {localized_role_label(&row_user.role)}
                                                </p>
                                            </div>
                                        </div>
                                        <div class="mt-3 grid grid-cols-2 gap-3 text-sm">
                                            <div>
                                                <p class="text-fg-muted">
                                                    {rust_i18n::t!(
                                                        "pages.admin_users.list.columns.full_name"
                                                    )}
                                                </p>
                                                <p class="text-fg">{row_user.full_name}</p>
                                            </div>
                                            <div>
                                                <p class="text-fg-muted">
                                                    {rust_i18n::t!(
                                                        "pages.admin_users.list.columns.system_admin"
                                                    )}
                                                </p>
                                                <p class="text-fg">
                                                    {if row_user.is_system_admin {
                                                        rust_i18n::t!(
                                                            "pages.admin_users.list.values.yes"
                                                        )
                                                    } else {
                                                        rust_i18n::t!(
                                                            "pages.admin_users.list.values.no"
                                                        )
                                                    }}
                                                </p>
                                            </div>
                                            <div>
                                                <p class="text-fg-muted">
                                                    {rust_i18n::t!(
                                                        "pages.admin_users.list.columns.mfa"
                                                    )}
                                                </p>
                                                <p class="text-fg">
                                                    {if row_user.mfa_enabled {
                                                        rust_i18n::t!(
                                                            "pages.admin_users.list.values.enabled"
                                                        )
                                                    } else {
                                                        rust_i18n::t!(
                                                            "pages.admin_users.list.values.disabled"
                                                        )
                                                    }}
                                                </p>
                                            </div>
                                            <div>
                                                <p class="text-fg-muted">
                                                    {rust_i18n::t!(
                                                        "pages.admin_users.list.columns.status"
                                                    )}
                                                </p>
                                                <p class="text-fg">
                                                    {if row_user.is_locked {
                                                        rust_i18n::t!(
                                                            "pages.admin_users.list.values.locked"
                                                        )
                                                    } else {
                                                        rust_i18n::t!(
                                                            "pages.admin_users.list.values.active"
                                                        )
                                                    }}
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
                                        {rust_i18n::t!("pages.admin_users.list.columns.username")}
                                    </th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">
                                        {rust_i18n::t!("pages.admin_users.list.columns.full_name")}
                                    </th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">
                                        {rust_i18n::t!("pages.admin_users.list.columns.role")}
                                    </th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">
                                        {rust_i18n::t!("pages.admin_users.list.columns.system_admin")}
                                    </th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">
                                        {rust_i18n::t!("pages.admin_users.list.columns.mfa")}
                                    </th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">
                                        {rust_i18n::t!("pages.admin_users.list.columns.status")}
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
                                                    {localized_role_label(&row_user.role)}
                                                </td>
                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-fg">
                                                    {if row_user.is_system_admin {
                                                        rust_i18n::t!(
                                                            "pages.admin_users.list.values.yes"
                                                        )
                                                    } else {
                                                        rust_i18n::t!(
                                                            "pages.admin_users.list.values.no"
                                                        )
                                                    }}
                                                </td>
                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-fg">
                                                    {if row_user.mfa_enabled {
                                                        rust_i18n::t!(
                                                            "pages.admin_users.list.values.enabled"
                                                        )
                                                    } else {
                                                        rust_i18n::t!(
                                                            "pages.admin_users.list.values.disabled"
                                                        )
                                                    }}
                                                </td>
                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-fg">
                                                    {if row_user.is_locked {
                                                        rust_i18n::t!(
                                                            "pages.admin_users.list.values.locked"
                                                        )
                                                    } else {
                                                        rust_i18n::t!(
                                                            "pages.admin_users.list.values.active"
                                                        )
                                                    }}
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
    use crate::test_support::{helpers::set_test_locale, ssr::render_to_string};

    fn user() -> UserResponse {
        UserResponse {
            id: "u1".into(),
            username: "alice".into(),
            full_name: "Alice Example".into(),
            role: "admin".into(),
            is_system_admin: true,
            mfa_enabled: false,
            is_locked: false,
            locked_until: None,
            failed_login_attempts: 0,
            password_expiry_warning_days: None,
        }
    }

    #[test]
    fn user_list_renders_empty_message() {
        let _locale = set_test_locale("ja");
        let html = render_to_string(move || {
            let users = Resource::new(
                || (true, 0u32),
                |_| async move {
                    Ok(PiiProtectedResponse {
                        data: Vec::new(),
                        pii_masked: false,
                    })
                },
            );
            users.set(Ok(PiiProtectedResponse {
                data: Vec::new(),
                pii_masked: false,
            }));
            let loading = Signal::derive(|| false);
            let on_select = Callback::new(|_| {});
            view! { <UserList users_resource=users loading=loading on_select=on_select /> }
        });
        assert!(html.contains(rust_i18n::t!("pages.admin_users.list.empty").as_ref()));
    }

    #[test]
    fn user_list_renders_rows() {
        let _locale = set_test_locale("ja");
        let html = render_to_string(move || {
            let users = Resource::new(
                || (true, 0u32),
                |_| async move {
                    Ok(PiiProtectedResponse {
                        data: vec![user()],
                        pii_masked: false,
                    })
                },
            );
            users.set(Ok(PiiProtectedResponse {
                data: vec![user()],
                pii_masked: false,
            }));
            let loading = Signal::derive(|| false);
            let on_select = Callback::new(|_| {});
            view! { <UserList users_resource=users loading=loading on_select=on_select /> }
        });
        assert!(html.contains("alice"));
        assert!(html.contains("Alice Example"));
    }
}
