use crate::pages::admin::{
    components::{
        attendance::AdminAttendanceToolsSection, holidays::HolidayManagementSection,
        requests::AdminRequestsSection, system_tools::AdminMfaResetSection,
        weekly_holidays::WeeklyHolidaySection,
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

    let vm = use_admin_view_model();

    view! {
        <layout::AdminDashboardScaffold admin_allowed=admin_allowed>
            <WeeklyHolidaySection
                state=vm.weekly_holiday_state
                resource=vm.weekly_holidays_resource
                action=vm.create_weekly_action
                reload=vm.reload_weekly
                message=vm.weekly_action_message
                error=vm.weekly_action_error
                admin_allowed=admin_allowed
                system_admin_allowed=system_admin_allowed
            />
            <div class="grid grid-cols-1 gap-6 lg:grid-cols-3">
                <AdminRequestsSection
                    users=vm.users_resource
                    filter=vm.requests_filter
                    resource=vm.requests_resource
                    action=vm.request_action
                    action_error=vm.requests_action_error
                    reload=vm.reload_requests
                />
                <AdminAttendanceToolsSection
                    repository=vm.repository.clone()
                    system_admin_allowed=system_admin_allowed
                    users=vm.users_resource
                />
                <AdminMfaResetSection
                    repository=vm.repository.clone()
                    system_admin_allowed=system_admin_allowed
                    users=vm.users_resource
                />
            </div>
            <HolidayManagementSection
                repository=vm.repository.clone()
                admin_allowed=admin_allowed
            />
        </layout::AdminDashboardScaffold>
    }
}
