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
        <div class="bg-surface-elevated shadow rounded-lg p-4 flex flex-col gap-3 lg:flex-row lg:items-end">
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
                class="w-full lg:w-auto px-4 py-2 bg-surface-muted text-fg rounded hover:bg-action-ghost-bg-hover"
                on:click=move |ev| on_select_current_month.call(ev)
            >
                {"今月"}
            </button>
            <button
                class="w-full lg:w-auto px-4 py-2 bg-action-primary-bg text-action-primary-text rounded disabled:opacity-50"
                disabled={move || history_loading.get()}
                on:click=move |ev| on_load_range.call(ev)
            >
                {move || if history_loading.get() { "読み込み中..." } else { "読み込み" }}
            </button>
            <button
                class="w-full lg:w-auto px-4 py-2 bg-action-secondary-bg text-action-secondary-text rounded disabled:opacity-50"
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
            <div class="bg-status-error-bg border border-status-error-border text-status-error-text px-4 py-3 rounded">
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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::api::ApiError;
    use crate::test_support::ssr::render_to_string;

    #[test]
    fn range_form_renders_buttons_and_errors() {
        let html = render_to_string(move || {
            let from = create_rw_signal("2025-01-01".to_string());
            let to = create_rw_signal("2025-01-31".to_string());
            let (exporting, _) = create_signal(false);
            let export_error = create_rw_signal(Some(ApiError::unknown("export failed")));
            let export_success = create_rw_signal(Some("ok".to_string()));
            let (history_loading, _) = create_signal(false);
            let (history_error, _) = create_signal(Some(ApiError::unknown("history failed")));
            let range_error = create_rw_signal(Some("range invalid".to_string()));
            let (last_refresh_error, _) = create_signal(Some(ApiError::unknown("refresh failed")));
            view! {
                <RangeFormSection
                    from_input=from
                    to_input=to
                    exporting=exporting.into()
                    export_error=export_error.read_only()
                    export_success=export_success.read_only()
                    history_loading=history_loading.into()
                    history_error=history_error.into()
                    range_error=range_error.read_only()
                    last_refresh_error=last_refresh_error.into()
                    on_select_current_month=Callback::new(|_| {})
                    on_load_range=Callback::new(|_| {})
                    on_export_csv=Callback::new(|_| {})
                />
            }
        });
        assert!(html.contains("CSVダウンロード"));
        assert!(html.contains("export failed"));
        assert!(html.contains("range invalid"));
        assert!(html.contains("history failed"));
        assert!(html.contains("refresh failed"));
    }
}
