use crate::pages::requests::components::status_label::{request_kind_title, request_status_label};
use crate::pages::requests::types::RequestSummary;
use leptos::ev::KeyboardEvent;
use leptos::html;
use leptos::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[component]
pub fn RequestDetailModal(selected: RwSignal<Option<RequestSummary>>) -> impl IntoView {
    let header_close_ref = create_node_ref::<html::Button>();
    let footer_close_ref = create_node_ref::<html::Button>();
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (&header_close_ref, &footer_close_ref);
    #[cfg(target_arch = "wasm32")]
    let previously_focused = create_rw_signal(None::<web_sys::HtmlElement>);

    let on_dialog_keydown = move |ev: KeyboardEvent| match ev.key().as_str() {
        "Escape" => {
            ev.prevent_default();
            selected.set(None);
            #[cfg(target_arch = "wasm32")]
            if let Some(element) = previously_focused.get_untracked() {
                let _ = element.focus();
                previously_focused.set(None);
            }
        }
        "Tab" => {
            #[cfg(target_arch = "wasm32")]
            {
                let active_id = web_sys::window()
                    .and_then(|window| window.document())
                    .and_then(|document| document.active_element())
                    .and_then(|element| element.get_attribute("id"))
                    .unwrap_or_default();
                if ev.shift_key() && active_id == "request-detail-modal-header-close" {
                    ev.prevent_default();
                    if let Some(button) = footer_close_ref.get() {
                        let _ = button.focus();
                    }
                } else if !ev.shift_key() && active_id == "request-detail-modal-footer-close" {
                    ev.prevent_default();
                    if let Some(button) = header_close_ref.get() {
                        let _ = button.focus();
                    }
                }
            }
        }
        _ => {}
    };

    create_effect(move |_| {
        if selected.get().is_some() {
            #[cfg(target_arch = "wasm32")]
            {
                let active = web_sys::window()
                    .and_then(|window| window.document())
                    .and_then(|document| document.active_element())
                    .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok());
                previously_focused.set(active);
                if let Some(button) = header_close_ref.get() {
                    let _ = button.focus();
                }
            }
        }
    });

    view! {
        <Show when=move || selected.get().is_some()>
            {move || {
                selected
                    .get()
                    .map(|summary| {
                        view! {
                            <div class="fixed inset-0 z-50 flex items-end sm:items-center justify-center">
                                <div
                                    class="fixed inset-0 bg-overlay-backdrop"
                                    on:click=move |_| {
                                        selected.set(None);
                                        #[cfg(target_arch = "wasm32")]
                                        if let Some(element) = previously_focused.get_untracked() {
                                            let _ = element.focus();
                                            previously_focused.set(None);
                                        }
                                    }
                                ></div>
                                <div
                                    class="relative bg-surface-elevated rounded-lg shadow-xl w-full max-w-md mx-4 p-6 space-y-4 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-action-primary-focus"
                                    role="dialog"
                                    aria-modal="true"
                                    tabindex="-1"
                                    on:keydown=on_dialog_keydown
                                >
                                    <div class="flex items-center justify-between">
                                        <div>
                                            <p class="text-sm text-fg-muted">{rust_i18n::t!("pages.requests.detail_modal.subtitle")}</p>
                                            <p class="text-lg font-semibold text-fg">
                                                {request_kind_title(summary.kind)}
                                            </p>
                                        </div>
                                        <button
                                            id="request-detail-modal-header-close"
                                            node_ref=header_close_ref
                                            aria-label={rust_i18n::t!("common.actions.close")}
                                            class="text-fg-muted hover:text-fg"
                                            on:click=move |_| {
                                                selected.set(None);
                                                #[cfg(target_arch = "wasm32")]
                                                if let Some(element) = previously_focused.get_untracked() {
                                                    let _ = element.focus();
                                                    previously_focused.set(None);
                                                }
                                            }
                                        >
                                            {"✕"}
                                        </button>
                                    </div>
                                    <div class="space-y-2 text-sm text-fg">
                                        <div>
                                            <span class="font-medium text-fg-muted">{rust_i18n::t!("pages.requests.detail_modal.status")}{": "}</span>
                                            <span class="capitalize">{request_status_label(&summary.status)}</span>
                                        </div>
                                        <div>
                                            <span class="font-medium text-fg-muted">{rust_i18n::t!("pages.requests.detail_modal.period")}{": "}</span>
                                            <span>{summary.primary_label.clone().unwrap_or_else(|| "-".into())}</span>
                                        </div>
                                        <div>
                                            <span class="font-medium text-fg-muted">{rust_i18n::t!("pages.requests.detail_modal.secondary")}{": "}</span>
                                            <span>{summary.secondary_label.clone().unwrap_or_else(|| "-".into())}</span>
                                        </div>
                                        <div>
                                            <span class="font-medium text-fg-muted">{rust_i18n::t!("pages.requests.detail_modal.reason")}{": "}</span>
                                            <span>{summary.reason.clone().unwrap_or_else(|| rust_i18n::t!("pages.requests.detail_modal.empty_reason").into_owned())}</span>
                                        </div>
                                        <div>
                                            <span class="font-medium text-fg-muted">{rust_i18n::t!("pages.requests.detail_modal.submitted_at")}{": "}</span>
                                            <span>{summary.submitted_at.clone().unwrap_or_else(|| "-".into())}</span>
                                        </div>
                                    </div>
                                    <div class="flex justify-end">
                                        <button
                                            id="request-detail-modal-footer-close"
                                            node_ref=footer_close_ref
                                            class="px-4 py-2 rounded bg-surface-muted text-fg hover:bg-surface-elevated"
                                            on:click=move |_| {
                                                selected.set(None);
                                                #[cfg(target_arch = "wasm32")]
                                                if let Some(element) = previously_focused.get_untracked() {
                                                    let _ = element.focus();
                                                    previously_focused.set(None);
                                                }
                                            }
                                        >
                                            {rust_i18n::t!("common.actions.close")}
                                        </button>
                                    </div>
                                </div>
                            </div>
                        }
                    })
            }}
        </Show>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::test_support::{helpers::set_test_locale, ssr::render_to_string};
    use serde_json::json;

    #[test]
    fn request_detail_modal_renders_summary() {
        let _locale = set_test_locale("en");
        let html = render_to_string(move || {
            let summary = RequestSummary::from_leave(&json!({
                "id": "req-1",
                "status": "pending",
                "start_date": "2025-01-10",
                "end_date": "2025-01-12",
                "leave_type": "annual",
                "reason": "family",
                "created_at": "2025-01-05T10:00:00Z"
            }));
            let selected = create_rw_signal(Some(summary));
            view! { <RequestDetailModal selected=selected /> }
        });
        assert!(html.contains("Leave Request"));
        assert!(html.contains("role=\"dialog\""));
        assert!(html.contains("aria-modal=\"true\""));
        assert!(html.contains("aria-label=\"Close\""));
        assert!(html.contains("Pending"));
        assert!(html.contains("family"));
    }
}
