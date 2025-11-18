use crate::components::layout::Layout;
use crate::pages::admin::{attendance, holidays, layout, requests, system_tools, weekly_holidays};
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

    view! {
        <Layout>
            <Show
                when=move || admin_allowed.get()
                fallback=move || view! {
                    <layout::UnauthorizedMessage />
                }.into_view()
            >
                <layout::AdminDashboardFrame>
                    <weekly_holidays::WeeklyHolidaySection
                        admin_allowed=admin_allowed.clone()
                        system_admin_allowed=system_admin_allowed.clone()
                    />
                    <div class="grid grid-cols-1 gap-6 lg:grid-cols-3">
                        <requests::AdminRequestsSection admin_allowed=admin_allowed.clone() />
                        <attendance::AdminAttendanceToolsSection
                            system_admin_allowed=system_admin_allowed.clone()
                        />
                        <system_tools::AdminMfaResetSection
                            system_admin_allowed=system_admin_allowed.clone()
                        />
                    </div>
                    <holidays::HolidayManagementSection admin_allowed=admin_allowed.clone() />
                </layout::AdminDashboardFrame>
            </Show>
        </Layout>
    }
}
