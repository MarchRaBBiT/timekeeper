use crate::components::layout::Layout;
use leptos::*;

#[component]
pub fn UnauthorizedMessage() -> impl IntoView {
    view! {
        <div class="space-y-6">
            <div class="bg-surface-elevated shadow rounded-lg p-6">
                <p class="text-sm text-fg">
                    {rust_i18n::t!("pages.admin_settings.unauthorized")}
                </p>
            </div>
        </div>
    }
}

#[component]
pub fn AdminSettingsFrame(children: Children) -> impl IntoView {
    view! {
        <div class="space-y-6">
            <div>
                <h1 class="text-2xl font-bold text-fg">
                    {rust_i18n::t!("pages.admin_settings.title")}
                </h1>
                <p class="mt-1 text-sm text-fg-muted">
                    {rust_i18n::t!("pages.admin_settings.description")}
                </p>
            </div>
            {children()}
        </div>
    }
}

#[component]
pub fn AdminSettingsScaffold(admin_allowed: Memo<bool>, children: Children) -> impl IntoView {
    let content = store_value(children());
    view! {
        <Layout>
            <Show
                when=move || admin_allowed.get()
                fallback=move || view! { <UnauthorizedMessage /> }.into_view()
            >
                <AdminSettingsFrame>
                    {content.get_value().clone()}
                </AdminSettingsFrame>
            </Show>
        </Layout>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::test_support::{
        helpers::set_test_locale,
        ssr::{render_to_string, render_with_router_to_string},
    };

    #[test]
    fn unauthorized_message_renders_copy() {
        let _locale = set_test_locale("en");
        let html = render_to_string(move || view! { <UnauthorizedMessage /> });
        assert!(html.contains("administrator (admin)"));
    }

    #[test]
    fn admin_settings_frame_renders_header() {
        let _locale = set_test_locale("en");
        let html = render_to_string(move || {
            view! {
                <AdminSettingsFrame>
                    <div>{"child"}</div>
                </AdminSettingsFrame>
            }
        });
        assert!(html.contains("Admin Settings"));
        assert!(html.contains("child"));
    }

    #[test]
    fn admin_settings_scaffold_switches_content() {
        let _locale = set_test_locale("en");
        let allowed_html = render_with_router_to_string("http://localhost/", move || {
            let allowed = create_memo(|_| true);
            view! {
                <AdminSettingsScaffold admin_allowed=allowed>
                    <div>{"allowed"}</div>
                </AdminSettingsScaffold>
            }
        });
        assert!(allowed_html.contains("Admin Settings"));
        assert!(allowed_html.contains("allowed"));

        let denied_html = render_with_router_to_string("http://localhost/", move || {
            let allowed = create_memo(|_| false);
            view! {
                <AdminSettingsScaffold admin_allowed=allowed>
                    <div>{"denied"}</div>
                </AdminSettingsScaffold>
            }
        });
        assert!(denied_html.contains("administrator (admin)"));
    }
}
