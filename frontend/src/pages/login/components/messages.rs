use leptos::*;

#[component]
pub fn InlineErrorMessage(error: ReadSignal<Option<String>>) -> impl IntoView {
    view! {
        <Show when=move || error.get().is_some() fallback=|| ()>
            <div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded">
                {error.get().unwrap_or_default()}
            </div>
        </Show>
    }
}
