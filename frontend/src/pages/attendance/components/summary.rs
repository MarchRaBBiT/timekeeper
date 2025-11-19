use crate::{
    components::forms::AttendanceActionButtons,
    state::attendance::{describe_holiday_reason, AttendanceState},
};
use leptos::*;

#[component]
pub fn SummarySection(
    state: ReadSignal<AttendanceState>,
    set_state: WriteSignal<AttendanceState>,
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
                <AttendanceActionButtons attendance_state=state set_attendance_state=set_state />
            </div>
        </Show>
    }
}
