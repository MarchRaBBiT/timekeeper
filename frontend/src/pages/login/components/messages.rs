use leptos::*;

#[component]
pub fn InlineErrorMessage(error: ReadSignal<Option<String>>) -> impl IntoView {
    view! {
        <Show when=move || error.get().is_some() fallback=|| ()>
            <div class="bg-status-error-bg border border-status-error-border text-status-error-text px-4 py-3 rounded">
                {error.get().unwrap_or_default()}
            </div>
        </Show>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::test_support::ssr::render_to_string;

    #[test]
    fn inline_error_message_renders_text() {
        let html = render_to_string(move || {
            let error = create_rw_signal(Some("ログインに失敗しました".to_string()));
            view! { <InlineErrorMessage error=error.read_only() /> }
        });
        assert!(html.contains("ログインに失敗しました"));
    }

    #[test]
    fn inline_error_message_hidden_without_error() {
        let html = render_to_string(move || {
            let error = create_rw_signal(Option::<String>::None);
            view! { <InlineErrorMessage error=error.read_only() /> }
        });
        assert!(!html.contains("bg-status-error-bg"));
    }
}
