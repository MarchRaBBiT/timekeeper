use leptos::*;

#[component]
pub fn GlobalFilters() -> impl IntoView {
    view! {
        <div class="flex items-center gap-3 text-sm text-gray-700 bg-white border rounded-lg px-4 py-2 shadow-sm">
            <span class="font-medium text-gray-900">{"フィルター"}</span>
            <span class="text-gray-500">{"現在の絞り込み条件は未設定です。今後の拡張で申請種別や期間を選択できるようにします。"} </span>
        </div>
    }
}
