use crate::components::layout::{ErrorMessage, LoadingSpinner};
use crate::pages::requests::types::{RequestKind, RequestSummary};
use leptos::*;

#[component]
pub fn RequestsList(
    summaries: Signal<Vec<RequestSummary>>,
    loading: Signal<bool>,
    error: Signal<Option<String>>,
    on_select: Callback<RequestSummary>,
    on_edit: Callback<RequestSummary>,
    on_cancel: Callback<RequestSummary>,
    message: RwSignal<crate::pages::requests::utils::MessageState>,
) -> impl IntoView {
    view! {
        <div class="bg-white shadow rounded-lg">
            <div class="px-6 py-4 border-b">
                <h3 class="text-lg font-medium text-gray-900">{"申請一覧"}</h3>
                <Show when=move || message.get().error.is_some()>
                    <div class="mt-2">
                        <ErrorMessage message={message.get().error.clone().unwrap_or_default()} />
                    </div>
                </Show>
                <Show when=move || message.get().success.is_some()>
                    <div class="mt-2 text-sm text-green-700 bg-green-50 border border-green-200 rounded px-3 py-2">
                        {message.get().success.clone().unwrap_or_default()}
                    </div>
                </Show>
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
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"操作"}</th>
                            </tr>
                        </thead>
                        <tbody class="bg-white divide-y divide-gray-200">
                            <For
                                each=move || summaries.get()
                                key=|summary| summary.id.clone()
                                children=move |summary: RequestSummary| {
                                    let on_select = on_select.clone();
                                    let summary = store_value(summary);
                                    let summary_value = summary.get_value();
                                    let status = summary_value.status.clone();
                                    let status_for_pending = status.clone();
                                    let status_for_not_pending = status.clone();
                                    let primary_label =
                                        summary_value.primary_label.clone().unwrap_or_else(|| "-".into());
                                    let secondary_label =
                                        summary_value.secondary_label.clone().unwrap_or_else(|| "-".into());
                                    let submitted_at =
                                        summary_value.submitted_at.clone().unwrap_or_else(|| "-".into());
                                    view! {
                                        <tr class="hover:bg-gray-50 cursor-pointer" on:click=move |_| on_select.call(summary.get_value())>
                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                                                {match summary_value.kind {
                                                    RequestKind::Leave => "休暇",
                                                    RequestKind::Overtime => "残業",
                                                }}
                                            </td>
                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                                                {primary_label.clone()}
                                            </td>
                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                                                {secondary_label.clone()}
                                            </td>
                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900 capitalize">
                                                {status.clone()}
                                            </td>
                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                                                {submitted_at.clone()}
                                            </td>
                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                                                <Show when=move || status_for_pending == "pending">
                                                    <div class="flex gap-2">
                                                        <button
                                                            class="text-blue-600 hover:underline"
                                                            on:click=move |ev| {
                                                                ev.stop_propagation();
                                                                on_edit.call(summary.get_value());
                                                            }
                                                        >
                                                            {"編集"}
                                                        </button>
                                                        <button
                                                            class="text-red-600 hover:underline"
                                                            on:click=move |ev| {
                                                                ev.stop_propagation();
                                                                on_cancel.call(summary.get_value());
                                                            }
                                                        >
                                                            {"取消"}
                                                        </button>
                                                    </div>
                                                </Show>
                                                <Show when=move || status_for_not_pending != "pending">
                                                    <span class="text-gray-400">{"-"}</span>
                                                </Show>
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
