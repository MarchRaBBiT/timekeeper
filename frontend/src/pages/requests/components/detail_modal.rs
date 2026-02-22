use crate::pages::requests::components::status_label::request_status_label;
use crate::pages::requests::types::{RequestKind, RequestSummary};
use leptos::*;

#[component]
pub fn RequestDetailModal(selected: RwSignal<Option<RequestSummary>>) -> impl IntoView {
    let on_close = { move |_| selected.set(None) };
    view! {
        <Show when=move || selected.get().is_some()>
            {move || {
                selected
                    .get()
                    .map(|summary| {
                        view! {
                            <div class="fixed inset-0 z-50 flex items-end sm:items-center justify-center">
                                <div class="fixed inset-0 bg-overlay-backdrop" on:click=on_close></div>
                                <div class="relative bg-surface-elevated rounded-lg shadow-xl w-full max-w-md mx-4 p-6 space-y-4">
                                    <div class="flex items-center justify-between">
                                        <div>
                                            <p class="text-sm text-fg-muted">{"申請の詳細"}</p>
                                            <p class="text-lg font-semibold text-fg">
                                                {match summary.kind {
                                                    RequestKind::Leave => "休暇申請",
                                                    RequestKind::Overtime => "残業申請",
                                                    RequestKind::AttendanceCorrection => "勤怠修正依頼",
                                                }}
                                            </p>
                                        </div>
                                        <button class="text-fg-muted hover:text-fg" on:click=on_close>
                                            {"✕"}
                                        </button>
                                    </div>
                                    <div class="space-y-2 text-sm text-fg">
                                        <div>
                                            <span class="font-medium text-fg-muted">{"ステータス: "}</span>
                                            <span class="capitalize">{request_status_label(&summary.status)}</span>
                                        </div>
                                        <div>
                                            <span class="font-medium text-fg-muted">{"期間/日付: "}</span>
                                            <span>{summary.primary_label.clone().unwrap_or_else(|| "-".into())}</span>
                                        </div>
                                        <div>
                                            <span class="font-medium text-fg-muted">{"補足: "}</span>
                                            <span>{summary.secondary_label.clone().unwrap_or_else(|| "-".into())}</span>
                                        </div>
                                        <div>
                                            <span class="font-medium text-fg-muted">{"理由: "}</span>
                                            <span>{summary.reason.clone().unwrap_or_else(|| "未入力".into())}</span>
                                        </div>
                                        <div>
                                            <span class="font-medium text-fg-muted">{"提出日: "}</span>
                                            <span>{summary.submitted_at.clone().unwrap_or_else(|| "-".into())}</span>
                                        </div>
                                    </div>
                                    <div class="flex justify-end">
                                        <button class="px-4 py-2 rounded bg-surface-muted text-fg hover:bg-surface-elevated" on:click=on_close>
                                            {"閉じる"}
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
    use crate::test_support::ssr::render_to_string;
    use serde_json::json;

    #[test]
    fn request_detail_modal_renders_summary() {
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
        assert!(html.contains("休暇申請"));
        assert!(html.contains("承認待ち"));
        assert!(html.contains("family"));
    }
}
