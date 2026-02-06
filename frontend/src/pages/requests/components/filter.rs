use crate::pages::requests::utils::RequestFilterState;
use leptos::*;

#[component]
pub fn RequestsFilter(filter_state: RequestFilterState) -> impl IntoView {
    let status_signal = filter_state.status_signal();
    view! {
        <div class="bg-surface-elevated shadow rounded-lg p-4 flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
            <div>
                <h3 class="text-sm font-semibold text-fg">{"申請の絞り込み"}</h3>
                <p class="text-xs text-fg-muted">{"ステータスで一覧の表示を切り替えます。"} </p>
            </div>
            <div class="flex items-center gap-2">
                <select
                    class="border border-form-control-border bg-form-control-bg text-form-control-text rounded px-2 py-1 text-sm"
                    prop:value=move || status_signal.get()
                    on:change=move |ev| status_signal.set(event_target_value(&ev))
                >
                    <option value="">{"すべて"}</option>
                    <option value="pending">{"保留"}</option>
                    <option value="approved">{"承認済み"}</option>
                    <option value="rejected">{"却下"}</option>
                    <option value="cancelled">{"取消"}</option>
                </select>
                <button
                    class="text-sm text-link hover:text-link-hover underline"
                    on:click=move |_| status_signal.set(String::new())
                >
                    {"クリア"}
                </button>
            </div>
        </div>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::test_support::ssr::render_to_string;

    #[test]
    fn requests_filter_renders_labels_and_options() {
        let html = render_to_string(move || {
            let filter = RequestFilterState::default();
            view! { <RequestsFilter filter_state=filter /> }
        });
        assert!(html.contains("申請の絞り込み"));
        assert!(html.contains("保留"));
        assert!(html.contains("承認済み"));
        assert!(html.contains("却下"));
        assert!(html.contains("取消"));
        assert!(html.contains("クリア"));
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
