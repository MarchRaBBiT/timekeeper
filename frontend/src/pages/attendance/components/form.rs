use crate::components::forms::DatePicker;
use crate::components::{error::InlineErrorMessage, layout::SuccessMessage};
use leptos::{ev::MouseEvent, Callback, *};

#[derive(Clone)]
enum AttendanceFeedback {
    Error(crate::api::ApiError),
    Success(String),
}

fn resolve_attendance_feedback(
    export_error: Option<crate::api::ApiError>,
    export_success: Option<String>,
    range_error: Option<String>,
    history_error: Option<crate::api::ApiError>,
    last_refresh_error: Option<crate::api::ApiError>,
) -> Option<AttendanceFeedback> {
    if let Some(message) = range_error {
        return Some(AttendanceFeedback::Error(crate::api::ApiError::validation(
            message,
        )));
    }
    if let Some(error) = export_error {
        return Some(AttendanceFeedback::Error(error));
    }
    if let Some(error) = history_error {
        return Some(AttendanceFeedback::Error(error));
    }
    if let Some(error) = last_refresh_error {
        return Some(AttendanceFeedback::Error(error));
    }
    export_success.map(AttendanceFeedback::Success)
}

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
    let feedback = Signal::derive(move || {
        resolve_attendance_feedback(
            export_error.get(),
            export_success.get(),
            range_error.get(),
            history_error.get(),
            last_refresh_error.get(),
        )
    });

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
        {move || match feedback.get() {
            Some(AttendanceFeedback::Error(err)) => {
                let signal = create_rw_signal(Some(err));
                view! { <InlineErrorMessage error={signal.into()} /> }.into_view()
            }
            Some(AttendanceFeedback::Success(message)) => {
                view! { <SuccessMessage message=message /> }.into_view()
            }
            None => ().into_view(),
        }}
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
        assert!(html.contains("range invalid"));
        assert!(!html.contains("export failed"));
        assert!(!html.contains("history failed"));
        assert!(!html.contains("refresh failed"));
    }

    #[test]
    fn feedback_resolution_prefers_errors_and_falls_back_to_success() {
        let err = resolve_attendance_feedback(
            Some(ApiError::unknown("export failed")),
            Some("ok".to_string()),
            None,
            None,
            None,
        );
        assert!(matches!(err, Some(AttendanceFeedback::Error(_))));

        let success = resolve_attendance_feedback(None, Some("ok".to_string()), None, None, None);
        assert!(matches!(
            success,
            Some(AttendanceFeedback::Success(message)) if message == "ok"
        ));
    }
}
