use crate::{
    components::layout::{ErrorMessage, LoadingSpinner},
    pages::dashboard::{
        repository::DashboardSummary,
        utils::{format_days, format_hours},
    },
};
use leptos::*;

#[component]
pub fn SummarySection(summary: Resource<(), Result<DashboardSummary, String>>) -> impl IntoView {
    view! {
        <div class="bg-white shadow rounded-lg p-6 space-y-4">
            <div>
                <h3 class="text-base font-semibold text-gray-900">{"勤務サマリー"}</h3>
                <p class="text-sm text-gray-600">{"今月の勤務時間と日数のスナップショット"}</p>
            </div>
            <div>
                {move || match summary.get() {
                    None => view! {
                        <div class="flex items-center gap-2 text-sm text-gray-500">
                            <LoadingSpinner />
                            <span>{"勤怠サマリーを読み込み中..."}</span>
                        </div>
                    }.into_view(),
                    Some(Err(err)) => view! { <ErrorMessage message={err.clone()} /> }.into_view(),
                    Some(Ok(data)) => view! {
                        <div class="grid grid-cols-1 gap-4 lg:grid-cols-3">
                            <Metric label="総労働時間".to_string() value={format_hours(data.total_work_hours)} />
                            <Metric label="勤務日数".to_string() value={format_days(data.total_work_days)} />
                            <Metric label="平均日次労働時間".to_string() value={format_hours(data.average_daily_hours)} />
                        </div>
                    }.into_view(),
                }}
            </div>
        </div>
    }
}

#[component]
fn Metric(label: String, value: String) -> impl IntoView {
    view! {
        <div class="p-4 rounded-lg bg-gray-50 border border-gray-100">
            <p class="text-xs font-medium text-gray-500 uppercase tracking-wide">{label}</p>
            <p class="mt-2 text-2xl font-semibold text-gray-900">{value}</p>
        </div>
    }
}
