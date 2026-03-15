use crate::components::layout::Layout;
use crate::pages::admin::components::departments::DepartmentsPanel;
use leptos::*;

#[component]
pub fn AdminDepartmentsPage() -> impl IntoView {
    view! {
        <Layout>
            <div class="space-y-6">
                <DepartmentsPanel />
            </div>
        </Layout>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::test_support::helpers::{admin_user, provide_auth, set_test_locale};
    use crate::test_support::ssr::render_with_router_to_string;

    #[test]
    fn admin_departments_page_renders_for_admin() {
        let _locale = set_test_locale("ja");
        let html = render_with_router_to_string("http://localhost/admin/departments", move || {
            provide_auth(Some(admin_user(true)));
            view! { <AdminDepartmentsPage /> }
        });
        assert!(html.contains("部署管理"));
    }
}
