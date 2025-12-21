use crate::pages::admin::{
    components::{
        attendance::AdminAttendanceToolsSection, holidays::HolidayManagementSection,
        requests::AdminRequestsSection, system_tools::AdminMfaResetSection,
        weekly_holidays::WeeklyHolidaySection,
    },
    layout,
    repository::AdminRepository,
};
use crate::state::auth::use_auth;
use leptos::*;

#[component]
pub fn AdminPanel() -> impl IntoView {
    let (auth, _set_auth) = use_auth();
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
    let repository = store_value(AdminRepository::new());
    let users_repository = repository.get_value();
    let users_resource = create_resource(
        move || admin_allowed.get(),
        move |allowed| {
            let repo = users_repository.clone();
            async move {
                if allowed {
                    repo.fetch_users().await
                } else {
                    Ok(Vec::new())
                }
            }
        },
    );

    view! {
        <layout::AdminDashboardScaffold admin_allowed=admin_allowed>
            <WeeklyHolidaySection
                repository=repository.get_value()
                admin_allowed=admin_allowed
                system_admin_allowed=system_admin_allowed
            />
            <div class="grid grid-cols-1 gap-6 lg:grid-cols-3">
                <AdminRequestsSection
                    repository=repository.get_value()
                    admin_allowed=admin_allowed
                    users=users_resource
                />
                <AdminAttendanceToolsSection
                    repository=repository.get_value()
                    system_admin_allowed=system_admin_allowed
                    users=users_resource
                />
                <AdminMfaResetSection
                    repository=repository.get_value()
                    system_admin_allowed=system_admin_allowed
                    users=users_resource
                />
            </div>
            <HolidayManagementSection
                repository=repository.get_value()
                admin_allowed=admin_allowed
            />
        </layout::AdminDashboardScaffold>
    }
}
