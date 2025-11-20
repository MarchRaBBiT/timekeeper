use crate::pages::requests::types::{RequestKind, RequestSummary};
use leptos::*;

#[component]
pub fn RequestDetailModal(selected: RwSignal<Option<RequestSummary>>) -> impl IntoView {
    let on_close = {
        let selected = selected.clone();
        move |_| selected.set(None)
    };
    view! {
        <Show when=move || selected.get().is_some()>
            {move || {
                selected
                    .get()
                    .map(|summary| {
                        view! {
                            <div class="fixed inset-0 z-50 flex items-end sm:items-center justify-center">
                                <div class="fixed inset-0 bg-black/40" on:click=on_close.clone()></div>
                                <div class="relative bg-white rounded-lg shadow-xl w-full max-w-md mx-4 p-6 space-y-4">
                                    <div class="flex items-center justify-between">
                                        <div>
                                            <p class="text-sm text-gray-500">{"申請の詳細"}</p>
                                            <p class="text-lg font-semibold text-gray-900">
                                                {match summary.kind {
                                                    RequestKind::Leave => "休暇申請",
                                                    RequestKind::Overtime => "残業申請",
                                                }}
                                            </p>
                                        </div>
                                        <button class="text-gray-500 hover:text-gray-700" on:click=on_close.clone()>
                                            {"✕"}
                                        </button>
                                    </div>
                                    <div class="space-y-2 text-sm text-gray-700">
                                        <div>
                                            <span class="font-medium text-gray-500">{"ステータス: "}</span>
                                            <span class="capitalize">{summary.status.clone()}</span>
                                        </div>
                                        <div>
                                            <span class="font-medium text-gray-500">{"期間/日付: "}</span>
                                            <span>{summary.primary_label.clone().unwrap_or_else(|| "-".into())}</span>
                                        </div>
                                        <div>
                                            <span class="font-medium text-gray-500">{"補足: "}</span>
                                            <span>{summary.secondary_label.clone().unwrap_or_else(|| "-".into())}</span>
                                        </div>
                                        <div>
                                            <span class="font-medium text-gray-500">{"理由: "}</span>
                                            <span>{summary.reason.clone().unwrap_or_else(|| "未入力".into())}</span>
                                        </div>
                                        <div>
                                            <span class="font-medium text-gray-500">{"提出日: "}</span>
                                            <span>{summary.submitted_at.clone().unwrap_or_else(|| "-".into())}</span>
                                        </div>
                                    </div>
                                    <div class="flex justify-end">
                                        <button class="px-4 py-2 rounded bg-gray-200 text-gray-700 hover:bg-gray-300" on:click=on_close.clone()>
                                            {"閉じる"}
                                        </button>
                                    </div>
                                </div>
                            </div>
                        }
                    })
            }}
        </Show>
    }
}
