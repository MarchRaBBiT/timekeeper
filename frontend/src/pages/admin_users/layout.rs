use leptos::*;

#[component]
pub fn UnauthorizedAdminUsersMessage() -> impl IntoView {
    view! {
        <div class="space-y-6">
            <div class="bg-surface-elevated shadow rounded-lg p-6">
                <p class="text-sm text-fg">
                    {rust_i18n::t!("pages.admin_users.layout.unauthorized")}
                </p>
            </div>
        </div>
    }
}

#[component]
pub fn AdminUsersFrame(children: Children) -> impl IntoView {
    view! {
        <div class="space-y-6">
            <div>
                <h1 class="text-2xl font-bold text-fg">
                    {rust_i18n::t!("pages.admin_users.layout.title")}
                </h1>
                <p class="mt-1 text-sm text-fg-muted">
                    {rust_i18n::t!("pages.admin_users.layout.description")}
                </p>
            </div>
            {children()}
        </div>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::test_support::{helpers::set_test_locale, ssr::render_to_string};

    #[test]
    fn unauthorized_message_renders_copy() {
        let _locale = set_test_locale("ja");
        let html = render_to_string(move || view! { <UnauthorizedAdminUsersMessage /> });
        assert!(html.contains(rust_i18n::t!("pages.admin_users.layout.unauthorized").as_ref()));
    }

    #[test]
    fn admin_users_frame_renders_header() {
        let _locale = set_test_locale("ja");
        let html = render_to_string(move || {
            view! {
                <AdminUsersFrame>
                    <div>{"child"}</div>
                </AdminUsersFrame>
            }
        });
        assert!(html.contains(rust_i18n::t!("pages.admin_users.layout.title").as_ref()));
        assert!(html.contains("child"));
    }
}
