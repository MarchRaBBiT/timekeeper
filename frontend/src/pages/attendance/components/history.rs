use crate::{
    api::{AttendanceResponse, BreakRecordResponse, HolidayCalendarEntry},
    components::layout::{ErrorMessage, LoadingSpinner},
    pages::attendance::repository,
    state::attendance::describe_holiday_reason,
};
use leptos::*;
use log::error;

#[component]
pub fn HistorySection(
    history: Signal<Vec<AttendanceResponse>>,
    holiday_entries: Signal<Vec<HolidayCalendarEntry>>,
    loading: Signal<bool>,
    error: Signal<Option<String>>,
) -> impl IntoView {
    view! {
        <div class="bg-white shadow overflow-hidden sm:rounded-md space-y-3 p-4">
            <Show when=move || error.get().is_some()>
                <ErrorMessage message={error.get().unwrap_or_default()} />
            </Show>
            <Show when=move || loading.get()>
                <div class="flex items-center gap-2 text-sm text-gray-500">
                    <LoadingSpinner />
                    <span>{"勤怠履歴を読み込み中..."}</span>
                </div>
            </Show>
            <Show when=move || !loading.get() && history.get().is_empty()>
                <p class="text-sm text-gray-500">{"該当期間の勤怠データはありません。期間を変更して再検索してください。"} </p>
            </Show>
            <Show when=move || !loading.get() && !history.get().is_empty()>
                <ul class="divide-y divide-gray-200">
                    <For
                        each=move || history.get()
                        key=|item| item.id.clone()
                        children=move |item| view! { <HistoryRow item=item holiday_entries=holiday_entries /> }
                    />
                </ul>
            </Show>
        </div>
    }
}

#[component]
fn HistoryRow(
    item: AttendanceResponse,
    holiday_entries: Signal<Vec<HolidayCalendarEntry>>,
) -> impl IntoView {
    let expanded = create_rw_signal(false);
    let breaks = create_rw_signal(Vec::<BreakRecordResponse>::new());
    let day_holiday_reason = create_memo(move |_| {
        holiday_entries
            .get()
            .iter()
            .find(|entry| entry.date == item.date)
            .map(|entry| entry.reason.clone())
    });

    let toggle = {
        let expanded = expanded.clone();
        let breaks = breaks.clone();
        let attendance_id = item.id.clone();
        move |_| {
            let now_expanded = !expanded.get();
            expanded.set(now_expanded);
            if now_expanded {
                let breaks = breaks.clone();
                let attendance_id = attendance_id.clone();
                spawn_local(async move {
                    match repository::fetch_breaks_by_attendance(&attendance_id).await {
                        Ok(list) => breaks.set(list),
                        Err(err) => error!("Failed to load breaks: {}", err),
                    }
                });
            }
        }
    };

    view! {
        <li class="px-2 py-4 sm:px-6">
            <div class="flex items-center justify-between">
                <div>
                    <div class="text-sm font-medium text-gray-900">{item.date.to_string()}</div>
                    <Show when=move || day_holiday_reason.get().is_some()>
                        <span class="inline-flex items-center mt-1 px-2 py-0.5 rounded-full text-xs bg-amber-50 text-amber-800">
                            {move || {
                                day_holiday_reason
                                    .get()
                                    .as_ref()
                                    .map(|code| describe_holiday_reason(code.trim()).to_string())
                                    .unwrap_or_default()
                            }}
                        </span>
                    </Show>
                    <div class="text-sm text-gray-500">
                        {match (item.clock_in_time, item.clock_out_time) {
                            (Some(start), Some(end)) => format!("{} - {}", start.format("%H:%M"), end.format("%H:%M")),
                            (Some(start), None) => format!("{} -", start.format("%H:%M")),
                            _ => "-".to_string(),
                        }}
                    </div>
                </div>
                <div class="flex items-center space-x-4">
                    <div class="text-sm font-medium text-gray-900">
                        {item.total_work_hours.map(|h| format!("{:.1}時間", h)).unwrap_or_default()}
                    </div>
                    <button
                        class="text-sm text-blue-600 hover:text-blue-800"
                        on:click=toggle
                    >
                        {move || if expanded.get() { "詳細を閉じる" } else { "詳細を見る" }}
                    </button>
                </div>
            </div>
            <Show when=move || expanded.get()>
                <div class="mt-4 bg-gray-50 rounded-lg p-4">
                    <h4 class="text-sm font-semibold text-gray-900">{"休憩記録"}</h4>
                    <Show when=move || breaks.get().is_empty()>
                        <p class="text-sm text-gray-600">{"休憩の記録はありません。"} </p>
                    </Show>
                    <Show when=move || !breaks.get().is_empty()>
                        <ul class="mt-2 space-y-1 text-sm text-gray-700">
                            <For
                                each=move || breaks.get()
                                key=|record| record.id.clone()
                                children=move |record| {
                                    let duration = record
                                        .duration_minutes
                                        .map(|mins| format!("{mins}分"))
                                        .unwrap_or_else(|| "-".into());
                                    view! {
                                        <li class="flex justify-between">
                                            <span>{format!(
                                                "{} - {}",
                                                record.break_start_time.format("%H:%M"),
                                                record
                                                    .break_end_time
                                                    .map(|t| t.format("%H:%M").to_string())
                                                    .unwrap_or_else(|| "-".into())
                                            )}</span>
                                            <span>{duration}</span>
                                        </li>
                                    }
                                }
                            />
                        </ul>
                    </Show>
                </div>
            </Show>
        </li>
    }
}
