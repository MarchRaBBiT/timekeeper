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
        <div class="bg-white shadow-premium rounded-3xl overflow-hidden border border-gray-100/50">
            <div class="px-6 py-4 border-b border-gray-100 flex items-center justify-between">
                <h3 class="text-lg font-display font-bold text-slate-900">{"勤怠履歴"}</h3>
                <Show when=move || loading.get()>
                    <div class="flex items-center gap-2 text-xs font-bold text-brand-600 uppercase tracking-widest animate-pulse">
                        <LoadingSpinner />
                        <span>{"更新中..."}</span>
                    </div>
                </Show>
            </div>
            <div class="p-2">
                <Show when=move || error.get().is_some()>
                    <div class="p-4">
                        <ErrorMessage message={error.get().unwrap_or_default()} />
                    </div>
                </Show>
                <Show when=move || !loading.get() && history.get().is_empty()>
                    <div class="p-12 text-center">
                        <div class="w-16 h-16 bg-slate-50 rounded-full flex items-center justify-center mx-auto mb-4 text-slate-300">
                            <i class="fas fa-calendar-times text-2xl"></i>
                        </div>
                        <p class="text-slate-500 font-medium font-sans">{"指定された期間の記録はありません"} </p>
                    </div>
                </Show>
                <Show when=move || !history.get().is_empty()>
                    <ul class="divide-y divide-gray-50">
                        <For
                            each=move || history.get()
                            key=|item| item.id.clone()
                            children=move |item| view! { <HistoryRow item=item holiday_entries=holiday_entries /> }
                        />
                    </ul>
                </Show>
            </div>
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
    let (loading_breaks, set_loading_breaks) = create_signal(false);

    let day_holiday_reason = create_memo(move |_| {
        holiday_entries
            .get()
            .iter()
            .find(|entry| entry.date == item.date)
            .map(|entry| entry.reason.clone())
    });

    let toggle = {
        let attendance_id = item.id.clone();
        move |_| {
            let now_expanded = !expanded.get();
            expanded.set(now_expanded);
            if now_expanded && breaks.get().is_empty() {
                let api =
                    use_context::<crate::api::ApiClient>().expect("ApiClient should be provided");
                let attendance_id = attendance_id.clone();
                set_loading_breaks.set(true);
                spawn_local(async move {
                    match repository::fetch_breaks_by_attendance(&api, &attendance_id).await {
                        Ok(list) => {
                            breaks.set(list);
                            set_loading_breaks.set(false);
                        }
                        Err(err) => {
                            error!("Failed to load breaks: {}", err);
                            set_loading_breaks.set(false);
                        }
                    }
                });
            }
        }
    };

    view! {
        <li class="group transition-all duration-200 hover:bg-slate-50/80 rounded-2xl mx-1 my-1">
            <div class="px-4 py-4 sm:px-6 cursor-pointer" on:click=toggle>
                <div class="flex items-center justify-between">
                    <div class="flex items-center gap-4">
                        <div class="w-12 h-12 rounded-xl bg-white border border-gray-100 shadow-sm flex flex-col items-center justify-center group-hover:border-brand-100 group-hover:bg-brand-50/30 transition-colors">
                            <span class="text-[10px] font-bold text-slate-400 leading-none mb-1">
                                {item.date.format("%m").to_string()}
                            </span>
                            <span class="text-lg font-display font-black text-slate-900 leading-none">
                                {item.date.format("%d").to_string()}
                            </span>
                        </div>
                        <div>
                            <div class="flex items-center gap-2">
                                <Show when=move || day_holiday_reason.get().is_some()>
                                    <span class="inline-flex items-center px-2 py-0.5 rounded-md text-[10px] font-bold bg-amber-100 text-amber-700 uppercase">
                                        {move || {
                                            day_holiday_reason
                                                .get()
                                                .as_ref()
                                                .map(|code| describe_holiday_reason(code.trim()).to_string())
                                                .unwrap_or_default()
                                        }}
                                    </span>
                                </Show>
                                <span class="text-sm font-bold text-slate-700">
                                    {match (item.clock_in_time, item.clock_out_time) {
                                        (Some(start), Some(end)) => format!("{} - {}", start.format("%H:%M"), end.format("%H:%M")),
                                        (Some(start), None) => format!("{} - (未退勤)", start.format("%H:%M")),
                                        _ => "記録なし".to_string(),
                                    }}
                                </span>
                            </div>
                        </div>
                    </div>
                    <div class="flex items-center gap-6">
                        <div class="text-right">
                            <p class="text-xs font-bold text-slate-400 uppercase tracking-wider mb-0.5">{"勤務時間"}</p>
                            <p class="text-lg font-display font-black text-brand-600">
                                {item.total_work_hours.map(|h| format!("{:.1}h", h)).unwrap_or_else(|| "-".into())}
                            </p>
                        </div>
                        <div class=move || format!("transition-transform duration-300 {}", if expanded.get() { "rotate-180 text-brand-600" } else { "text-slate-300" })>
                            <i class="fas fa-chevron-down"></i>
                        </div>
                    </div>
                </div>

                <Show when=move || expanded.get()>
                    <div class="mt-4 bg-white rounded-2xl border border-gray-100 p-4 shadow-sm animate-pop-in overflow-hidden">
                        <div class="flex items-center justify-between mb-3">
                            <h4 class="text-xs font-bold text-slate-500 uppercase tracking-widest flex items-center gap-2">
                                <i class="fas fa-mug-hot text-amber-400"></i>
                                {"休憩詳細"}
                            </h4>
                            <Show when=move || loading_breaks.get()>
                                <div class="animate-spin rounded-full h-3 w-3 border-b-2 border-brand-600"></div>
                            </Show>
                        </div>

                        <Show when=move || !loading_breaks.get() && breaks.get().is_empty()>
                            <p class="text-xs text-slate-400 font-medium py-2">{"休憩の記録はありません"}</p>
                        </Show>

                        <Show when=move || !breaks.get().is_empty()>
                            <ul class="space-y-2">
                                <For
                                    each=move || breaks.get()
                                    key=|record| record.id.clone()
                                    children=move |record| {
                                        let duration = record
                                            .duration_minutes
                                            .map(|mins| format!("{mins}分"))
                                            .unwrap_or_else(|| "-".into());
                                        view! {
                                            <li class="flex items-center justify-between p-2 rounded-xl bg-slate-50/50 text-xs font-medium">
                                                <span class="text-slate-600">{format!(
                                                    "{} - {}",
                                                    record.break_start_time.format("%H:%M"),
                                                    record
                                                        .break_end_time
                                                        .map(|t| t.format("%H:%M").to_string())
                                                        .unwrap_or_else(|| "進行中".into())
                                                )}</span>
                                                <span class="text-slate-900 font-bold">{duration}</span>
                                            </li>
                                        }
                                    }
                                />
                            </ul>
                        </Show>
                    </div>
                </Show>
            </div>
        </li>
    }
}
