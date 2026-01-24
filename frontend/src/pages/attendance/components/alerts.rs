use crate::{
    api::HolidayCalendarEntry,
    components::{error::InlineErrorMessage, layout::LoadingSpinner},
    state::attendance::describe_holiday_reason,
};
use leptos::*;

#[component]
pub fn HolidayAlerts(
    holiday_entries: Signal<Vec<HolidayCalendarEntry>>,
    loading: Signal<bool>,
    error: Signal<Option<crate::api::ApiError>>,
    active_period: Signal<(i32, u32)>,
    on_refresh: Callback<()>,
) -> impl IntoView {
    view! {
        <div class="bg-surface-elevated shadow rounded-lg p-4 space-y-3">
            <div class="flex flex-col gap-1 lg:flex-row lg:items-center lg:justify-between">
                <div>
                    <h3 class="text-base font-semibold text-fg">{"今月の休日"}</h3>
                    <p class="text-sm text-fg-muted">
                        {move || {
                            let (year, month) = active_period.get();
                            format!("{year}年{month}月に登録済みの休日一覧です。")
                        }}
                    </p>
                </div>
                <button
                    class="px-3 py-1 text-sm rounded border border-border text-fg hover:bg-action-ghost-bg_hover disabled:opacity-50"
                    disabled={move || loading.get()}
                    on:click=move |_| on_refresh.call(())
                >
                    {"再取得"}
                </button>
            </div>
            <Show when=move || error.get().is_some()>
                <InlineErrorMessage error={error} />
            </Show>
            <Show when=move || loading.get()>
                <div class="flex items-center gap-2 text-sm text-fg-muted">
                    <LoadingSpinner />
                    <span>{"休日カレンダーを読み込み中..."}</span>
                </div>
            </Show>
            <Show when=move || !loading.get() && holiday_entries.get().is_empty()>
                <p class="text-sm text-fg-muted">{"登録された休日はありません。必要に応じて管理者へ連絡してください。"} </p>
            </Show>
            <Show when=move || !loading.get() && !holiday_entries.get().is_empty()>
                <ul class="space-y-2">
                    <For
                        each=move || holiday_entries.get()
                        key=|entry| (entry.date, entry.reason.clone())
                        children=move |entry: HolidayCalendarEntry| {
                            let label = describe_holiday_reason(entry.reason.as_str());
                            view! {
                                <li class="flex items-center justify-between text-sm text-fg">
                                    <span>{entry.date.format("%Y-%m-%d").to_string()}</span>
                                    <span class="px-2 py-0.5 rounded-full text-xs bg-status-info-bg text-status-info-text">
                                        {label}
                                    </span>
                                </li>
                            }
                        }
                    />
                </ul>
            </Show>
        </div>
    }
}
