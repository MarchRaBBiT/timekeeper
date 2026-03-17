use crate::pages::admin::{
    components::requests::AdminRequestsSection, layout, view_model::use_admin_view_model,
};
use crate::state::auth::use_auth;
use leptos::*;

#[component]
pub fn AdminPanel() -> impl IntoView {
    let (auth, _) = use_auth();
    let admin_allowed = create_memo(move |_| {
        auth.get()
            .user
            .as_ref()
            .map(|user| {
                user.is_system_admin
                    || user.role.eq_ignore_ascii_case("manager")
                    || user.role.eq_ignore_ascii_case("admin")
            })
            .unwrap_or(false)
    });

    let vm = use_admin_view_model();

    view! {
        <layout::AdminDashboardScaffold admin_allowed=admin_allowed>
            <AdminRequestsSection
                users=vm.users_resource
                filter=vm.requests_filter
                resource=vm.requests_resource
                action=vm.request_action
                action_error=vm.requests_action_error
                reload=vm.reload_requests
            />
        </layout::AdminDashboardScaffold>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::test_support::helpers::{admin_user, manager_user, provide_auth, set_test_locale};
    use crate::test_support::ssr::render_with_router_to_string;

    #[test]
    fn admin_panel_renders_for_admin() {
        let _locale = set_test_locale("ja");
        let html = render_with_router_to_string("http://localhost/", move || {
            provide_auth(Some(admin_user(true)));
            view! { <AdminPanel /> }
        });
        assert!(html.contains(rust_i18n::t!("pages.admin.title").as_ref()));
        assert!(html.contains("申請一覧"));
    }

    #[test]
    fn admin_panel_renders_for_manager() {
        let _locale = set_test_locale("ja");
        let html = render_with_router_to_string("http://localhost/", move || {
            provide_auth(Some(manager_user()));
            view! { <AdminPanel /> }
        });
        assert!(html.contains("申請一覧"));
    }
}
