use crate::pages::admin::components::{
    attendance::AdminAttendanceToolsSection, holidays::HolidayManagementSection,
    subject_requests::AdminSubjectRequestsSection, system_tools::AdminMfaResetSection,
    weekly_holidays::WeeklyHolidaySection,
};
use crate::pages::admin_settings::{layout, view_model::use_admin_settings_view_model};
use crate::state::auth::use_auth;
use leptos::*;

#[component]
pub fn AdminSettingsPanel() -> impl IntoView {
    let (auth, _) = use_auth();
    let admin_allowed = create_memo(move |_| {
        auth.get()
            .user
            .as_ref()
            .map(|user| user.is_system_admin || user.role.eq_ignore_ascii_case("admin"))
            .unwrap_or(false)
    });
    let system_admin_allowed = create_memo(move |_| {
        auth.get()
            .user
            .as_ref()
            .map(|user| user.is_system_admin)
            .unwrap_or(false)
    });

    let vm = use_admin_settings_view_model();
    let repo_attendance = store_value(vm.repository.clone());
    let repo_mfa = store_value(vm.repository.clone());
    let repo_holiday = store_value(vm.repository);

    view! {
        <layout::AdminSettingsScaffold admin_allowed=admin_allowed>
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
            <div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
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
            <HolidayManagementSection
                repository=repo_holiday.get_value()
                admin_allowed=admin_allowed
            />
        </layout::AdminSettingsScaffold>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::test_support::helpers::{admin_user, provide_auth, regular_user, set_test_locale};
    use crate::test_support::ssr::render_with_router_to_string;

    #[test]
    fn admin_settings_panel_renders_for_admin() {
        let _locale = set_test_locale("ja");
        let html = render_with_router_to_string("http://localhost/", move || {
            provide_auth(Some(admin_user(true)));
            view! { <AdminSettingsPanel /> }
        });
        assert!(html.contains(rust_i18n::t!("pages.admin_settings.title").as_ref()));
        assert!(html
            .contains(rust_i18n::t!("admin_components.weekly_holidays.fields.starts_on").as_ref()));
    }

    #[test]
    fn admin_settings_panel_shows_unauthorized_for_non_admin() {
        let _locale = set_test_locale("ja");
        let html = render_with_router_to_string("http://localhost/", move || {
            provide_auth(Some(regular_user()));
            view! { <AdminSettingsPanel /> }
        });
        assert!(html.contains(rust_i18n::t!("pages.admin_settings.unauthorized").as_ref()));
        assert!(!html
            .contains(rust_i18n::t!("admin_components.weekly_holidays.fields.starts_on").as_ref()));
    }
}
