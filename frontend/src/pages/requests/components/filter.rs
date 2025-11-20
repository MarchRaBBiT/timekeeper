use crate::pages::requests::utils::RequestFilterState;
use leptos::*;

#[component]
pub fn RequestsFilter(filter_state: RequestFilterState) -> impl IntoView {
    let status_signal = filter_state.status_signal();
    view! {
        <div class="bg-white shadow rounded-lg p-4 flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
            <div>
                <h3 class="text-sm font-semibold text-gray-900">{"申請の絞り込み"}</h3>
                <p class="text-xs text-gray-600">{"ステータスで一覧の表示を切り替えます。"} </p>
            </div>
            <div class="flex items-center gap-2">
                <select
                    class="border rounded px-2 py-1 text-sm"
                    prop:value=move || status_signal.get()
                    on:change=move |ev| status_signal.set(event_target_value(&ev))
                >
                    <option value="">{"すべて"}</option>
                    <option value="pending">{"保留"}</option>
                    <option value="approved">{"承認済み"}</option>
                    <option value="rejected">{"却下"}</option>
                    <option value="cancelled">{"取消"}</option>
                </select>
                <button
                    class="text-sm text-gray-700 underline"
                    on:click=move |_| status_signal.set(String::new())
                >
                    {"クリア"}
                </button>
            </div>
        </div>
    }
}
