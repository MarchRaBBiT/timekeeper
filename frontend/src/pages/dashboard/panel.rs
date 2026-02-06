use crate::components::forms::AttendanceActionButtons;
use crate::pages::dashboard::{
    components::{ActivitiesSection, AlertsSection, Clock, GlobalFilters, SummarySection},
    layout::DashboardFrame,
    view_model::use_dashboard_view_model,
};
use leptos::*;

#[component]
pub fn DashboardPage() -> impl IntoView {
    let vm = use_dashboard_view_model();
    let (attendance_state, _) = vm.attendance_state;

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
                    <Clock />
                    <AttendanceActionButtons
                        attendance_state=attendance_state
                        action_pending={vm.clock_action.pending()}
                        message={vm.clock_message.read_only()}
                        on_clock_in={Callback::new(vm.handle_clock_in())}
                        on_clock_out={Callback::new(vm.handle_clock_out())}
                        on_break_start={Callback::new(vm.handle_break_start())}
                        on_break_end={Callback::new(vm.handle_break_end())}
                    />
                </div>
            </div>
        </DashboardFrame>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::api::test_support::mock::*;
    use crate::api::ApiClient;
    use crate::test_support::ssr::with_local_runtime_async;
    use serde_json::json;

    fn mock_server() -> MockServer {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/attendance/status");
            then.status(200).json_body(json!({
                "status": "clocked_in",
                "attendance_id": "att-1",
                "active_break_id": null,
                "clock_in_time": "2025-01-01T09:00:00",
                "clock_out_time": null
            }));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/attendance/me");
            then.status(200).json_body(json!([]));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/holidays/check");
            then.status(200).json_body(json!({
                "is_holiday": false,
                "reason": null
            }));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/attendance/me/summary");
            then.status(200).json_body(json!({
                "month": 1,
                "year": 2025,
                "total_work_hours": 160.0,
                "total_work_days": 20,
                "average_daily_hours": 8.0
            }));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/requests/me");
            then.status(200).json_body(json!({
                "leave_requests": [],
                "overtime_requests": []
            }));
        });
        server
    }

    #[test]
    fn dashboard_page_renders_sections() {
        with_local_runtime_async(|| async {
            let runtime = leptos::create_runtime();
            let server = mock_server();
            provide_context(ApiClient::new_with_base_url(&server.url("/api")));

            leptos_reactive::suppress_resource_load(true);
            let html = view! { <DashboardPage /> }
                .into_view()
                .render_to_string()
                .to_string();
            leptos_reactive::suppress_resource_load(false);

            assert!(html.contains("フィルター"));
            assert!(html.contains("出勤"));

            runtime.dispose();
        });
    }
}
