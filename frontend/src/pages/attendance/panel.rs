use super::{
    components::{
        alerts::HolidayAlerts, form::RangeFormSection, history::HistorySection,
        summary::SummarySection,
    },
    layout::AttendanceFrame,
    view_model::use_attendance_view_model,
};
use leptos::*;

#[component]
pub fn AttendancePage() -> impl IntoView {
    view! { <AttendancePanel /> }
}

#[component]
pub fn AttendancePanel() -> impl IntoView {
    let vm = use_attendance_view_model();
    let (state, _) = vm.state;
    let form_state = vm.form_state.clone();
    let from_input = form_state.start_date_signal();
    let to_input = form_state.end_date_signal();

    let history_loading = vm.history_resource.loading();
    let history_error =
        Signal::derive(move || vm.history_resource.get().and_then(|result| result.err()));

    let holiday_loading = vm.holiday_resource.loading();
    let holiday_entries = Signal::derive(move || {
        vm.holiday_resource
            .get()
            .and_then(|result| result.ok())
            .unwrap_or_default()
    });
    let holiday_error =
        Signal::derive(move || vm.holiday_resource.get().and_then(|result| result.err()));
    let active_holiday_period =
        Signal::derive(move || vm.holiday_query.with(|query| (query.year, query.month)));

    let exporting = vm.export_action.pending();
    let last_refresh_error = Signal::derive(move || state.with(|s| s.last_refresh_error.clone()));
    let history_signal = Signal::derive(move || state.with(|s| s.attendance_history.clone()));

    view! {
        <AttendanceFrame>
            <div class="space-y-6">
                <SummarySection
                    state=state
                    action_pending={vm.clock_action.pending()}
                    message={vm.clock_message.read_only()}
                    on_clock_in={Callback::new(vm.handle_clock_in())}
                    on_clock_out={Callback::new(vm.handle_clock_out())}
                    on_break_start={Callback::new(vm.handle_break_start())}
                    on_break_end={Callback::new(vm.handle_break_end())}
                />
                <RangeFormSection
                    from_input=from_input
                    to_input=to_input
                    exporting={exporting.into()}
                    export_error={vm.export_error.read_only()}
                    export_success={vm.export_success.read_only()}
                    history_loading=history_loading
                    history_error={history_error}
                    range_error={vm.range_error.read_only()}
                    last_refresh_error=last_refresh_error
                    on_select_current_month=Callback::new(vm.on_select_current_month())
                    on_load_range=Callback::new(vm.on_load_range())
                    on_export_csv=Callback::new(vm.on_export_csv())
                />
                <HolidayAlerts
                    holiday_entries={holiday_entries}
                    loading={holiday_loading}
                    error={holiday_error}
                    active_period={active_holiday_period}
                    on_refresh=Callback::new(vm.on_refresh_holidays())
                />
                <HistorySection
                    history=history_signal
                    holiday_entries={holiday_entries}
                    loading=history_loading
                    error={history_error}
                />
            </div>
        </AttendanceFrame>
    }
}
