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
