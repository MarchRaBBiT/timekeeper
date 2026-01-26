use crate::{
    api::{AttendanceResponse, BreakRecordResponse, HolidayCalendarEntry},
    components::{error::InlineErrorMessage, layout::LoadingSpinner},
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
    error: Signal<Option<crate::api::ApiError>>,
) -> impl IntoView {
    view! {
        <div class="bg-surface-elevated shadow-premium rounded-3xl overflow-hidden border border-border">
            <div class="px-6 py-4 border-b border-border flex items-center justify-between">
                <h3 class="text-lg font-display font-bold text-fg">{"勤怠履歴"}</h3>
                <Show when=move || loading.get()>
                    <div class="flex items-center gap-2 text-xs font-bold text-action-primary-bg uppercase tracking-widest animate-pulse">
                        <LoadingSpinner />
                        <span>{"更新中..."}</span>
                    </div>
                </Show>
            </div>
            <div class="p-2">
                <Show when=move || error.get().is_some()>
                    <div class="p-4">
                        <InlineErrorMessage error={error} />
                    </div>
                </Show>
                <Show when=move || !loading.get() && history.get().is_empty()>
                    <div class="p-12 text-center">
                        <div class="w-16 h-16 bg-surface-muted rounded-full flex items-center justify-center mx-auto mb-4 text-fg-muted">
                            <i class="fas fa-calendar-times text-2xl"></i>
                        </div>
                        <p class="text-fg-muted font-medium font-sans">{"指定された期間の記録はありません"} </p>
                    </div>
                </Show>
                <Show when=move || !history.get().is_empty()>
                    <ul class="divide-y divide-border">
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
                let api = use_context::<crate::api::ApiClient>()
                    .unwrap_or_else(crate::api::ApiClient::new);
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
        <li class="group transition-all duration-200 hover:bg-surface-muted rounded-2xl mx-1 my-1">
            <div class="px-4 py-4 sm:px-6 cursor-pointer" on:click=toggle>
                <div class="flex items-center justify-between">
                    <div class="flex items-center gap-4">
                        <div class="w-12 h-12 rounded-xl bg-surface-elevated border border-border shadow-sm flex flex-col items-center justify-center group-hover:border-action-primary-border group-hover:bg-primary-subtle transition-colors">
                            <span class="text-[10px] font-bold text-fg-muted leading-none mb-1">
                                {item.date.format("%m").to_string()}
                            </span>
                            <span class="text-lg font-display font-black text-fg leading-none">
                                {item.date.format("%d").to_string()}
                            </span>
                        </div>
                        <div>
                            <div class="flex items-center gap-2">
                                <Show when=move || day_holiday_reason.get().is_some()>
                                    <span class="inline-flex items-center px-2 py-0.5 rounded-md text-[10px] font-bold bg-status-warning-bg text-status-warning-text uppercase">
                                        {move || {
                                            day_holiday_reason
                                                .get()
                                                .as_ref()
                                                .map(|code| describe_holiday_reason(code.trim()).to_string())
                                                .unwrap_or_default()
                                        }}
                                    </span>
                                </Show>
                                <span class="text-sm font-bold text-fg">
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
                            <p class="text-xs font-bold text-fg-muted uppercase tracking-wider mb-0.5">{"勤務時間"}</p>
                            <p class="text-lg font-display font-black text-action-primary-bg">
                                {item.total_work_hours.map(|h| format!("{:.1}h", h)).unwrap_or_else(|| "-".into())}
                            </p>
                        </div>
                        <div class=move || format!("transition-transform duration-300 {}", if expanded.get() { "rotate-180 text-action-primary-bg" } else { "text-fg-muted" })>
                            <i class="fas fa-chevron-down"></i>
                        </div>
                    </div>
                </div>

                <Show when=move || expanded.get()>
                    <div class="mt-4 bg-surface-elevated rounded-2xl border border-border p-4 shadow-sm animate-pop-in overflow-hidden">
                        <div class="flex items-center justify-between mb-3">
                            <h4 class="text-xs font-bold text-fg-muted uppercase tracking-widest flex items-center gap-2">
                                <i class="fas fa-mug-hot text-status-warning-text"></i>
                                {"休憩詳細"}
                            </h4>
                            <Show when=move || loading_breaks.get()>
                                <div class="animate-spin rounded-full h-3 w-3 border-b-2 border-action-primary-bg"></div>
                            </Show>
                        </div>

                        <Show when=move || !loading_breaks.get() && breaks.get().is_empty()>
                            <p class="text-xs text-fg-muted font-medium py-2">{"休憩の記録はありません"}</p>
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
                                            <li class="flex items-center justify-between p-2 rounded-xl bg-surface-muted text-xs font-medium">
                                                <span class="text-fg-muted">{format!(
                                                    "{} - {}",
                                                    record.break_start_time.format("%H:%M"),
                                                    record
                                                        .break_end_time
                                                        .map(|t| t.format("%H:%M").to_string())
                                                        .unwrap_or_else(|| "進行中".into())
                                                )}</span>
                                                <span class="text-fg font-bold">{duration}</span>
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
