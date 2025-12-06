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
    let auth_for_admin = auth.clone();
    let admin_allowed = create_memo(move |_| {
        auth_for_admin
            .get()
            .user
            .as_ref()
            .map(|user| user.is_system_admin || user.role.eq_ignore_ascii_case("admin"))
            .unwrap_or(false)
    });
    let auth_for_system = auth.clone();
    let system_admin_allowed = create_memo(move |_| {
        auth_for_system
            .get()
            .user
            .as_ref()
            .map(|user| user.is_system_admin)
            .unwrap_or(false)
    });
    let repository = store_value(AdminRepository::new());
    let users_repository = repository.get_value();
    let users_allowed = admin_allowed.clone();
    let users_resource = create_resource(
        move || users_allowed.get(),
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
        <layout::AdminDashboardScaffold admin_allowed=admin_allowed.clone()>
            <WeeklyHolidaySection
                repository=repository.get_value()
                admin_allowed=admin_allowed.clone()
                system_admin_allowed=system_admin_allowed.clone()
            />
            <div class="grid grid-cols-1 gap-6 lg:grid-cols-3">
                <AdminRequestsSection
                    repository=repository.get_value()
                    admin_allowed=admin_allowed.clone()
                    users=users_resource.clone()
                />
                <AdminAttendanceToolsSection
                    repository=repository.get_value()
                    system_admin_allowed=system_admin_allowed.clone()
                    users=users_resource.clone()
                />
                <AdminMfaResetSection
                    repository=repository.get_value()
                    system_admin_allowed=system_admin_allowed.clone()
                    users=users_resource.clone()
                />
            </div>
            <HolidayManagementSection
                repository=repository.get_value()
                admin_allowed=admin_allowed.clone()
            />
        </layout::AdminDashboardScaffold>
    }
}
