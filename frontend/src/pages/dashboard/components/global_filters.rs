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
            <span class="font-medium text-fg">{"フィルター"}</span>
            <label class="flex items-center gap-2">
                <span class="text-fg-muted">{"申請ステータス"}</span>
                <select
                    class="border border-form-control-border bg-form-control-bg text-form-control-text rounded px-2 py-1 text-sm"
                    on:change=on_change
                    prop:value={move || filter.get().as_value().to_string()}
                >
                    <option value="all">{"すべて"}</option>
                    <option value="pending">{"承認待ちのみ"}</option>
                    <option value="approved">{"承認済みのみ"}</option>
                </select>
            </label>
        </div>
    }
}
