use crate::components::layout::{ErrorMessage, LoadingSpinner};
use crate::pages::requests::types::{RequestKind, RequestSummary};
use leptos::*;

#[component]
pub fn RequestsList(
    summaries: Signal<Vec<RequestSummary>>,
    loading: Signal<bool>,
    error: Signal<Option<String>>,
    on_select: Callback<RequestSummary>,
) -> impl IntoView {
    view! {
        <div class="bg-white shadow rounded-lg">
            <div class="px-6 py-4 border-b">
                <h3 class="text-lg font-medium text-gray-900">{"申請一覧"}</h3>
            </div>
            <Show when=move || error.get().is_some()>
                <div class="px-6 py-4">
                    <ErrorMessage message={error.get().unwrap_or_default()} />
                </div>
            </Show>
            <Show when=move || loading.get()>
                <div class="px-6 py-4 flex items-center gap-2 text-sm text-gray-600">
                    <LoadingSpinner />
                    <span>{"申請情報を読み込み中です..."}</span>
                </div>
            </Show>
            <Show when=move || !loading.get() && summaries.get().is_empty() && error.get().is_none()>
                <div class="px-6 py-4 text-sm text-gray-600">
                    {"表示できる申請がありません。新しい申請を作成してください。"}
                </div>
            </Show>
            <Show when=move || !summaries.get().is_empty()>
                <div class="overflow-x-auto">
                    <table class="min-w-full divide-y divide-gray-200">
                        <thead class="bg-gray-50">
                            <tr>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"種類"}</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"期間/日付"}</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"補足"}</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"ステータス"}</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"提出日"}</th>
                            </tr>
                        </thead>
                        <tbody class="bg-white divide-y divide-gray-200">
                            <For
                                each=move || summaries.get()
                                key=|summary| summary.id.clone()
                                children=move |summary: RequestSummary| {
                                    let on_select = on_select.clone();
                                    let row = summary.clone();
                                    view! {
                                        <tr class="hover:bg-gray-50 cursor-pointer" on:click=move |_| on_select.call(row.clone())>
                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                                                {match summary.kind {
                                                    RequestKind::Leave => "休暇",
                                                    RequestKind::Overtime => "残業",
                                                }}
                                            </td>
                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                                                {summary.primary_label.clone().unwrap_or_else(|| "-".into())}
                                            </td>
                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                                                {summary.secondary_label.clone().unwrap_or_else(|| "-".into())}
                                            </td>
                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900 capitalize">
                                                {summary.status.clone()}
                                            </td>
                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                                                {summary.submitted_at.clone().unwrap_or_else(|| "-".into())}
                                            </td>
                                        </tr>
                                    }
                                }
                            />
                        </tbody>
                    </table>
                </div>
            </Show>
        </div>
    }
}
