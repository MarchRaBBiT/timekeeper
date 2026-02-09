use crate::api::ApiError;
use leptos::*;

#[component]
pub fn InlineErrorMessage(error: Signal<Option<ApiError>>) -> impl IntoView {
    view! {
        <Show when=move || error.get().is_some() fallback=|| ()>
            <div class="bg-status-error-bg border border-status-error-border text-status-error-text px-4 py-3 rounded space-y-1 my-2">
                <div class="font-bold">{move || error.get().map(|e| e.error).unwrap_or_default()}</div>
                {move || error.get().map(|e| {
                    let code = &e.code;
                    let details = e.details.as_ref();
                    if code == "VALIDATION_ERROR" {
                        if let Some(details) = details {
                            if let Some(errors) = details.get("errors").and_then(|v| v.as_array()) {
                                return view! {
                                    <ul class="list-disc list-inside text-sm">
                                        {errors.iter().map(|err| {
                                            view! { <li>{err.as_str().unwrap_or_default().to_string()}</li> }
                                        }).collect_view()}
                                    </ul>
                                }.into_view();
                            }
                        }
                    }
                    if code != "UNKNOWN" && !code.is_empty() {
                         view! { <div class="text-xs opacity-75">{"Code: "}{code.clone()}</div> }.into_view()
                    } else {
                        ().into_view()
                    }
                }).unwrap_or_else(|| ().into_view())}
            </div>
        </Show>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::test_support::ssr::render_to_string;
    use serde_json::json;

    #[test]
    fn inline_error_renders_validation_details() {
        let html = render_to_string(move || {
            let error = ApiError {
                error: "Validation failed".into(),
                code: "VALIDATION_ERROR".into(),
                details: Some(json!({
                    "errors": ["Name is required", "Email is invalid"]
                })),
            };
            let signal = create_rw_signal(Some(error));
            view! { <InlineErrorMessage error={signal.into()} /> }
        });
        assert!(html.contains("Validation failed"));
        assert!(html.contains("Name is required"));
        assert!(html.contains("Email is invalid"));
    }

    #[test]
    fn inline_error_renders_code_when_present() {
        let html = render_to_string(move || {
            let error = ApiError {
                error: "Request failed".into(),
                code: "REQUEST_FAILED".into(),
                details: None,
            };
            let signal = create_rw_signal(Some(error));
            view! { <InlineErrorMessage error={signal.into()} /> }
        });
        assert!(html.contains("Request failed"));
        assert!(html.contains("Code: REQUEST_FAILED"));
    }
}
