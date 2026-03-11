use crate::pages::requests::utils::RequestFilterState;
use leptos::*;

#[component]
pub fn RequestsFilter(filter_state: RequestFilterState) -> impl IntoView {
    let status_signal = filter_state.status_signal();
    view! {
        <div class="bg-surface-elevated shadow rounded-lg p-4 flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
            <div>
                <h3 class="text-sm font-semibold text-fg">{rust_i18n::t!("pages.requests.filter.title")}</h3>
                <p class="text-xs text-fg-muted">{rust_i18n::t!("pages.requests.filter.description")} </p>
            </div>
            <div class="flex items-center gap-2">
                <select
                    class="border border-form-control-border bg-form-control-bg text-form-control-text rounded px-2 py-1 text-sm"
                    prop:value=move || status_signal.get()
                    on:change=move |ev| status_signal.set(event_target_value(&ev))
                >
                    <option value="">{rust_i18n::t!("pages.requests.filter.options.all")}</option>
                    <option value="pending">{rust_i18n::t!("pages.requests.status.pending")}</option>
                    <option value="approved">{rust_i18n::t!("pages.requests.status.approved")}</option>
                    <option value="rejected">{rust_i18n::t!("pages.requests.status.rejected")}</option>
                    <option value="cancelled">{rust_i18n::t!("pages.requests.status.cancelled")}</option>
                </select>
                <button
                    class="text-sm text-link hover:text-link-hover underline"
                    on:click=move |_| status_signal.set(String::new())
                >
                    {rust_i18n::t!("common.actions.clear")}
                </button>
            </div>
        </div>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::test_support::{helpers::set_test_locale, ssr::render_to_string};

    #[test]
    fn requests_filter_renders_labels_and_options() {
        let _locale = set_test_locale("en");
        let html = render_to_string(move || {
            let filter = RequestFilterState::default();
            view! { <RequestsFilter filter_state=filter /> }
        });
        assert!(html.contains("Filter Requests"));
        assert!(html.contains("Pending"));
        assert!(html.contains("Approved"));
        assert!(html.contains("Rejected"));
        assert!(html.contains("Cancelled"));
        assert!(html.contains("Clear"));
    }

    #[test]
    fn requests_filter_reflects_current_filter_value() {
        let html = render_to_string(move || {
            let filter = RequestFilterState::default();
            filter.status_signal().set("pending".into());
            view! { <RequestsFilter filter_state=filter /> }
        });
        assert!(html.contains("pending"));
    }
}
