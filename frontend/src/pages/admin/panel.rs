use crate::pages::admin::{
    components::{
        attendance::AdminAttendanceToolsSection, holidays::HolidayManagementSection,
        requests::AdminRequestsSection, subject_requests::AdminSubjectRequestsSection,
        system_tools::AdminMfaResetSection, weekly_holidays::WeeklyHolidaySection,
    },
    layout,
    view_model::use_admin_view_model,
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
    let system_admin_allowed = create_memo(move |_| {
        auth.get()
            .user
            .as_ref()
            .map(|user| user.is_system_admin)
            .unwrap_or(false)
    });

    let vm = use_admin_view_model();
    let manager_only = vm.manager_only;
    let repo_attendance = store_value(vm.repository.clone());
    let repo_mfa = store_value(vm.repository.clone());
    let repo_holiday = store_value(vm.repository);

    view! {
        <layout::AdminDashboardScaffold admin_allowed=admin_allowed>
            <Show when=move || !manager_only.get()>
                <WeeklyHolidaySection
                    state=vm.weekly_holiday_state
                    resource=vm.weekly_holidays_resource
                    action=vm.create_weekly_action
                    delete_action=vm.delete_weekly_action
                    reload=vm.reload_weekly
                    message=vm.weekly_action_message
                    error=vm.weekly_action_error
                    admin_allowed=admin_allowed
                    system_admin_allowed=system_admin_allowed
                />
            </Show>
            <div class=move || if manager_only.get() {
                "grid grid-cols-1 gap-6"
            } else {
                "grid grid-cols-1 gap-6 lg:grid-cols-3"
            }>
                <AdminRequestsSection
                    users=vm.users_resource
                    filter=vm.requests_filter
                    resource=vm.requests_resource
                    action=vm.request_action
                    action_error=vm.requests_action_error
                    reload=vm.reload_requests
                />
                <Show when=move || !manager_only.get()>
                    <AdminAttendanceToolsSection
                        repository=repo_attendance.get_value()
                        system_admin_allowed=system_admin_allowed
                        users=vm.users_resource
                    />
                    <AdminMfaResetSection
                        repository=repo_mfa.get_value()
                        system_admin_allowed=system_admin_allowed
                        users=vm.users_resource
                    />
                </Show>
            </div>
            <Show when=move || system_admin_allowed.get()>
                <AdminSubjectRequestsSection
                    users=vm.users_resource
                    filter=vm.subject_request_filter
                    resource=vm.subject_requests_resource
                    action=vm.subject_request_action
                    action_error=vm.subject_request_action_error
                    reload=vm.reload_subject_requests
                />
            </Show>
            <Show when=move || !manager_only.get()>
                <HolidayManagementSection
                    repository=repo_holiday.get_value()
                    admin_allowed=admin_allowed
                />
            </Show>
        </layout::AdminDashboardScaffold>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::test_support::helpers::{admin_user, manager_user, provide_auth, set_test_locale};
    use crate::test_support::ssr::render_with_router_to_string;

    #[test]
    fn admin_panel_renders_sections_for_admin() {
        let _locale = set_test_locale("ja");
        let html = render_with_router_to_string("http://localhost/", move || {
            provide_auth(Some(admin_user(true)));
            view! { <AdminPanel /> }
        });
        assert!(html.contains(rust_i18n::t!("pages.admin.title").as_ref()));
        assert!(html
            .contains(rust_i18n::t!("admin_components.weekly_holidays.fields.starts_on").as_ref()));
    }

    #[test]
    fn admin_panel_shows_only_requests_section_for_manager() {
        let _locale = set_test_locale("ja");
        let html = render_with_router_to_string("http://localhost/", move || {
            provide_auth(Some(manager_user()));
            view! { <AdminPanel /> }
        });
        assert!(html.contains("申請一覧"));
        // WeeklyHolidaySection の稼働開始日フィールドが表示されないことを確認
        // (pages.admin.description に "週次休日" が含まれるため title では判定できない)
        assert!(!html
            .contains(rust_i18n::t!("admin_components.weekly_holidays.fields.starts_on").as_ref()));
    }
}
