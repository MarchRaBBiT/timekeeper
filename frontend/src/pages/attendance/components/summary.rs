use crate::{
    components::forms::AttendanceActionButtons,
    state::attendance::{describe_holiday_reason, AttendanceState},
};
use leptos::{ev::MouseEvent, *};

#[component]
pub fn SummarySection(
    state: ReadSignal<AttendanceState>,
    action_pending: ReadSignal<bool>,
    message: ReadSignal<Option<String>>,
    on_clock_in: Callback<MouseEvent>,
    on_clock_out: Callback<MouseEvent>,
    on_break_start: Callback<MouseEvent>,
    on_break_end: Callback<MouseEvent>,
) -> impl IntoView {
    view! {
        <div>
            <h1 class="text-2xl font-bold text-gray-900">{"勤怠管理"}</h1>
            <p class="mt-1 text-sm text-gray-600">{"当日のステータスを確認できます。"}</p>
            <Show when=move || state.get().today_holiday_reason.is_some()>
                <p class="mt-1 text-sm text-amber-700">
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
            <div class="rounded-md p-4 border bg-white shadow-sm">
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
