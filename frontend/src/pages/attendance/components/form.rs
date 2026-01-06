use crate::components::forms::DatePicker;
use crate::components::{error::InlineErrorMessage, layout::SuccessMessage};
use leptos::{ev::MouseEvent, Callback, *};

#[component]
pub fn RangeFormSection(
    from_input: RwSignal<String>,
    to_input: RwSignal<String>,
    exporting: Signal<bool>,
    export_error: ReadSignal<Option<crate::api::ApiError>>,
    export_success: ReadSignal<Option<String>>,
    history_loading: Signal<bool>,
    history_error: Signal<Option<crate::api::ApiError>>,
    range_error: ReadSignal<Option<String>>,
    last_refresh_error: Signal<Option<crate::api::ApiError>>,
    on_select_current_month: Callback<MouseEvent>,
    on_load_range: Callback<MouseEvent>,
    on_export_csv: Callback<MouseEvent>,
) -> impl IntoView {
    view! {
        <div class="bg-white shadow rounded-lg p-4 flex flex-col gap-3 lg:flex-row lg:items-end">
            <div class="w-full lg:w-48">
                <DatePicker
                    label=Some("開始日")
                    value=from_input
                />
            </div>
            <div class="w-full lg:w-48">
                <DatePicker
                    label=Some("終了日")
                    value=to_input
                />
            </div>
            <button
                class="w-full lg:w-auto px-4 py-2 bg-gray-200 text-gray-800 rounded hover:bg-gray-300"
                on:click=move |ev| on_select_current_month.call(ev)
            >
                {"今月"}
            </button>
            <button
                class="w-full lg:w-auto px-4 py-2 bg-blue-600 text-white rounded disabled:opacity-50"
                disabled={move || history_loading.get()}
                on:click=move |ev| on_load_range.call(ev)
            >
                {move || if history_loading.get() { "読み込み中..." } else { "読み込み" }}
            </button>
            <button
                class="w-full lg:w-auto px-4 py-2 bg-indigo-600 text-white rounded disabled:opacity-50"
                disabled={move || exporting.get()}
                on:click=move |ev| on_export_csv.call(ev)
            >
                {move || if exporting.get() { "CSV生成中..." } else { "CSVダウンロード" }}
            </button>
        </div>
        <Show when=move || export_error.get().is_some()>
            <InlineErrorMessage error={export_error.into()} />
        </Show>
        <Show when=move || export_success.get().is_some()>
            <SuccessMessage message={export_success.get().unwrap_or_default()} />
        </Show>
        <Show when=move || range_error.get().is_some()>
            <div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded">
                {range_error.get().unwrap_or_default()}
            </div>
        </Show>
        <Show when=move || history_error.get().is_some()>
            <InlineErrorMessage error={history_error} />
        </Show>
        <Show when=move || last_refresh_error.get().is_some()>
            <InlineErrorMessage error={last_refresh_error} />
        </Show>
    }
}
