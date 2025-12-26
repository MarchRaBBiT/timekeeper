use crate::components::forms::AttendanceActionButtons;
use crate::pages::dashboard::{
    components::{ActivitiesSection, AlertsSection, GlobalFilters, SummarySection},
    layout::DashboardFrame,
    view_model::use_dashboard_view_model,
};
use leptos::*;

#[component]
pub fn DashboardPage() -> impl IntoView {
    let vm = use_dashboard_view_model();
    let (attendance_state, set_attendance_state) = vm.attendance_state;

    view! {
        <DashboardFrame>
            <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
                <div class="lg:col-span-2 space-y-6">
                    <SummarySection summary={vm.summary_resource} />
                    <AlertsSection alerts={vm.alerts_resource} />
                    <div class="space-y-4">
                        <GlobalFilters filter={vm.activity_filter} />
                        <ActivitiesSection activities={vm.activities_resource} />
                    </div>
                </div>
                <div class="space-y-6">
                    <AttendanceActionButtons
                        attendance_state=attendance_state
                        set_attendance_state=set_attendance_state
                    />
                </div>
            </div>
        </DashboardFrame>
    }
}
