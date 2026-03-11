use crate::pages::dashboard::utils::ActivityStatusFilter;
use leptos::*;
use wasm_bindgen::JsCast;

#[component]
pub fn GlobalFilters(filter: RwSignal<ActivityStatusFilter>) -> impl IntoView {
    let on_change = move |ev: web_sys::Event| {
        if let Some(target) = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok())
        {
            let next = ActivityStatusFilter::from_str(&target.value());
            filter.set(next);
        }
    };

    view! {
        <div class="flex items-center gap-3 text-sm text-fg bg-surface-elevated border border-border rounded-lg px-4 py-2 shadow-sm">
            <span class="font-medium text-fg">{rust_i18n::t!("pages.dashboard.filters.title")}</span>
            <label class="flex items-center gap-2">
                <span class="text-fg-muted">{rust_i18n::t!("pages.dashboard.filters.status")}</span>
                <select
                    class="border border-form-control-border bg-form-control-bg text-form-control-text rounded px-2 py-1 text-sm"
                    on:change=on_change
                    prop:value={move || filter.get().as_value().to_string()}
                >
                    <option value="all">{rust_i18n::t!("pages.dashboard.filters.options.all")}</option>
                    <option value="pending">{rust_i18n::t!("pages.dashboard.filters.options.pending")}</option>
                    <option value="approved">{rust_i18n::t!("pages.dashboard.filters.options.approved")}</option>
                </select>
            </label>
        </div>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::test_support::ssr::render_to_string;

    #[test]
    fn global_filters_renders_options() {
        let html = render_to_string(move || {
            let filter = create_rw_signal(ActivityStatusFilter::All);
            view! { <GlobalFilters filter=filter /> }
        });
        assert!(html.contains("option"));
    }
}
