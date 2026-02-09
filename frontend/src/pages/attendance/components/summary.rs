use crate::{
    components::forms::AttendanceActionButtons,
    state::attendance::{describe_holiday_reason, AttendanceState, ClockMessage},
};
use leptos::{ev::MouseEvent, *};

#[component]
pub fn SummarySection(
    state: ReadSignal<AttendanceState>,
    action_pending: ReadSignal<bool>,
    message: ReadSignal<Option<ClockMessage>>,
    on_clock_in: Callback<MouseEvent>,
    on_clock_out: Callback<MouseEvent>,
    on_break_start: Callback<MouseEvent>,
    on_break_end: Callback<MouseEvent>,
) -> impl IntoView {
    view! {
        <div>
            <h1 class="text-2xl font-bold text-fg">{"勤怠管理"}</h1>
            <p class="mt-1 text-sm text-fg-muted">{"当日のステータスを確認できます。"}</p>
            <Show when=move || state.get().today_holiday_reason.is_some()>
                <p class="mt-1 text-sm text-status-warning-text">
                    {move || state
                        .get()
                        .today_holiday_reason
                        .as_ref()
                        .map(|reason| describe_holiday_reason(reason).to_string())
                        .unwrap_or_default()}
                </p>
            </Show>
        </div>
        <Show when=move || state.get().today_status.is_some()>
            <div class="rounded-md p-4 border border-border bg-surface-elevated shadow-sm">
                <AttendanceActionButtons
                    attendance_state=state
                    action_pending=action_pending
                    message=message
                    on_clock_in=on_clock_in
                    on_clock_out=on_clock_out
                    on_break_start=on_break_start
                    on_break_end=on_break_end
                />
            </div>
        </Show>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::api::AttendanceStatusResponse;
    use crate::state::attendance::AttendanceState;
    use crate::test_support::ssr::render_to_string;
    use chrono::NaiveDate;

    #[test]
    fn summary_section_renders_status_and_holiday() {
        let html = render_to_string(move || {
            let mut state = AttendanceState::default();
            state.today_holiday_reason = Some("public holiday".into());
            state.today_status = Some(AttendanceStatusResponse {
                status: "clocked_in".into(),
                attendance_id: Some("att-1".into()),
                active_break_id: None,
                clock_in_time: Some(
                    NaiveDate::from_ymd_opt(2025, 1, 1)
                        .unwrap()
                        .and_hms_opt(9, 0, 0)
                        .unwrap(),
                ),
                clock_out_time: None,
            });
            let (signal, _) = create_signal(state);
            let (pending, _) = create_signal(false);
            let message = create_rw_signal(None::<ClockMessage>);
            view! {
                <SummarySection
                    state=signal
                    action_pending=pending.into()
                    message=message.read_only()
                    on_clock_in=Callback::new(|_| {})
                    on_clock_out=Callback::new(|_| {})
                    on_break_start=Callback::new(|_| {})
                    on_break_end=Callback::new(|_| {})
                />
            }
        });
        assert!(html.contains("勤怠管理"));
        assert!(html.contains("祝日"));
    }
}
