use leptos::*;

#[component]
pub fn EmptyState(
    #[prop(into)] title: String,
    #[prop(optional, into)] description: Option<String>,
    #[prop(optional)] icon: Option<View>,
) -> impl IntoView {
    view! {
        <div class="text-center py-12 px-4 rounded-lg border-2 border-dashed border-gray-300 bg-gray-50 dark:bg-gray-800 dark:border-gray-700">
            <div class="mx-auto h-12 w-12 text-gray-400 dark:text-gray-500">
                {icon.unwrap_or_else(|| view! {
                    <svg class="mx-auto h-12 w-12 text-gray-400 dark:text-gray-500" fill="none" viewBox="0 0 24 24" stroke="currentColor" aria-hidden="true">
                        <path vector-effect="non-scaling-stroke" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 13h6m-3-3v6m-9 1V7a2 2 0 012-2h6l2 2h6a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2z" />
                    </svg>
                }.into_view())}
            </div>
            <h3 class="mt-2 text-sm font-semibold text-gray-900 dark:text-gray-100">{title}</h3>
            {move || description.clone().map(|desc| view! {
                <p class="mt-1 text-sm text-gray-500 dark:text-gray-400">{desc}</p>
            })}
        </div>
    }
}
