use crate::api::ApiError;
use crate::components::{
    empty_state::EmptyState, error::InlineErrorMessage, layout::LoadingSpinner,
};
use crate::pages::requests::types::{RequestKind, RequestSummary};
use leptos::ev::KeyboardEvent;
use leptos::*;

#[component]
pub fn RequestsList(
    summaries: Signal<Vec<RequestSummary>>,
    loading: Signal<bool>,
    error: Signal<Option<ApiError>>,
    on_select: Callback<RequestSummary>,
    on_edit: Callback<RequestSummary>,
    on_cancel: Callback<RequestSummary>,
    message: RwSignal<crate::pages::requests::utils::MessageState>,
) -> impl IntoView {
    view! {
        <div class="bg-surface-elevated shadow-premium rounded-3xl overflow-hidden border border-border">
            <div class="px-8 py-6 border-b border-border">
                <h3 class="text-xl font-display font-bold text-fg">{"申請一覧"}</h3>
                <Show when=move || message.get().error.is_some()>
                    <div class="mt-4">
                        <InlineErrorMessage error={Signal::derive(move || message.get().error).into()} />
                    </div>
                </Show>
                <Show when=move || message.get().success.is_some()>
                    <div class="mt-4 flex items-center gap-2 p-3 rounded-xl bg-status-success-bg border border-status-success-border text-status-success-text animate-pop-in">
                        <i class="fas fa-check-circle"></i>
                        <span class="text-sm font-medium">{message.get().success.clone().unwrap_or_default()}</span>
                    </div>
                </Show>
            </div>

            <Show when=move || error.get().is_some()>
                <div class="p-8">
                    <InlineErrorMessage error={error.into()} />
                </div>
            </Show>

            <Show when=move || loading.get()>
                <div class="p-12 flex flex-col items-center justify-center gap-4 text-fg-muted">
                    <LoadingSpinner />
                    <span class="text-sm font-medium tracking-widest uppercase">{"データを取得中..."}</span>
                </div>
            </Show>

            <Show when=move || !loading.get() && summaries.get().is_empty() && error.get().is_none()>
                <EmptyState
                    title="表示できる申請がありません"
                    description="左または上のフォームから新しい申請を送信できます"
                    icon=view! { <i class="fas fa-inbox text-4xl text-fg-muted"></i> }.into_view()
                />
            </Show>

            <Show when=move || !summaries.get().is_empty()>
                <div class="overflow-x-auto">
                    <table class="min-w-full divide-y divide-border">
                        <thead>
                            <tr class="bg-surface-muted">
                                <th class="px-8 py-4 text-left text-xs font-bold text-fg-muted uppercase tracking-widest">{"種類"}</th>
                                <th class="px-8 py-4 text-left text-xs font-bold text-fg-muted uppercase tracking-widest">{"期間 / 日付"}</th>
                                <th class="px-8 py-4 text-left text-xs font-bold text-fg-muted uppercase tracking-widest">{"補足"}</th>
                                <th class="px-8 py-4 text-left text-xs font-bold text-fg-muted uppercase tracking-widest">{"ステータス"}</th>
                                <th class="px-8 py-4 text-left text-xs font-bold text-fg-muted uppercase tracking-widest">{"提出日"}</th>
                                <th class="px-8 py-4 text-left text-xs font-bold text-fg-muted uppercase tracking-widest">{"操作"}</th>
                            </tr>
                        </thead>
                        <tbody class="divide-y divide-border bg-surface-elevated">
                            <For
                                each=move || summaries.get()
                                key=|summary| summary.id.clone()
                                children=move |summary: RequestSummary| {
                                    let summary = store_value(summary);
                                    let summary_value = summary.get_value();
                                    let status = summary_value.status.clone();
                                    let status_for_pending = status.clone();
                                    let status_for_not_pending = status.clone();
                                    let primary_label =
                                        summary_value.primary_label.clone().unwrap_or_else(|| "-".into());
                                    let secondary_label =
                                        summary_value.secondary_label.clone().unwrap_or_else(|| "-".into());
                                    let submitted_at =
                                        summary_value.submitted_at.clone().unwrap_or_else(|| "-".into());

                                    let status_style = match status.as_str() {
                                        "approved" => "bg-status-success-bg text-status-success-text",
                                        "rejected" => "bg-status-error-bg text-status-error-text",
                                        "pending" => "bg-status-warning-bg text-status-warning-text",
                                        _ => "bg-status-neutral-bg text-status-neutral-text",
                                    };

                                    view! {
                                        <tr
                                            class="hover:bg-surface-muted transition-colors group cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-action-primary-focus focus-visible:ring-inset"
                                            tabindex="0"
                                            role="button"
                                            on:click=move |_| on_select.call(summary.get_value())
                                            on:keydown=move |ev: KeyboardEvent| match ev.key().as_str() {
                                                "Enter" => on_select.call(summary.get_value()),
                                                " " | "Spacebar" => {
                                                    ev.prevent_default();
                                                    on_select.call(summary.get_value());
                                                }
                                                _ => {}
                                            }
                                        >
                                            <td class="px-8 py-5 whitespace-nowrap">
                                                <span class="inline-flex items-center px-2 py-0.5 rounded-md text-[10px] font-bold bg-status-neutral-bg text-status-neutral-text uppercase">
                                                    {match summary_value.kind {
                                                        RequestKind::Leave => "休暇",
                                                        RequestKind::Overtime => "残業",
                                                        RequestKind::AttendanceCorrection => "勤怠修正",
                                                    }}
                                                </span>
                                            </td>
                                            <td class="px-8 py-5 whitespace-nowrap text-sm font-bold text-fg">
                                                {primary_label.clone()}
                                            </td>
                                            <td class="px-8 py-5 whitespace-nowrap text-sm text-fg-muted">
                                                {secondary_label.clone()}
                                            </td>
                                            <td class="px-8 py-5 whitespace-nowrap">
                                                <span class=format!("inline-flex items-center px-2.5 py-0.5 rounded-full text-[10px] font-black uppercase tracking-wider {}", status_style)>
                                                    {status.clone()}
                                                </span>
                                            </td>
                                            <td class="px-8 py-5 whitespace-nowrap text-sm font-medium text-fg-muted">
                                                {submitted_at.clone()}
                                            </td>
                                            <td class="px-8 py-5 whitespace-nowrap text-sm text-fg">
                                                <Show when=move || status_for_pending == "pending">
                                                    <div class="flex gap-4">
                                                        <button
                                                            class="text-link hover:text-link-hover font-bold flex items-center gap-1 transition-colors"
                                                            on:click=move |ev| {
                                                                ev.stop_propagation();
                                                                on_edit.call(summary.get_value());
                                                            }
                                                        >
                                                            <i class="fas fa-edit text-xs"></i>
                                                            {"編集"}
                                                        </button>
                                                        <button
                                                            class="text-action-danger-bg hover:text-action-danger-bg-hover font-bold flex items-center gap-1 transition-colors"
                                                            on:click=move |ev| {
                                                                ev.stop_propagation();
                                                                on_cancel.call(summary.get_value());
                                                            }
                                                        >
                                                            <i class="fas fa-times-circle text-xs"></i>
                                                            {"取消"}
                                                        </button>
                                                    </div>
                                                </Show>
                                                <Show when=move || status_for_not_pending != "pending">
                                                    <span class="text-fg-muted">
                                                        <i class="fas fa-lock text-xs"></i>
                                                    </span>
                                                </Show>
                                            </td>
                                        </tr>
                                    }
                                }
                            />
                        </tbody>
                    </table>
                </div>
            </Show>
        </div>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::pages::requests::utils::MessageState;
    use crate::test_support::ssr::render_to_string;
    use serde_json::json;

    fn summary() -> RequestSummary {
        RequestSummary::from_leave(&json!({
            "id": "req-1",
            "status": "pending",
            "start_date": "2025-01-10",
            "end_date": "2025-01-12",
            "leave_type": "annual",
            "reason": "family",
            "created_at": "2025-01-05T10:00:00Z"
        }))
    }

    #[test]
    fn requests_list_renders_empty_state() {
        let html = render_to_string(move || {
            let (summaries, _) = create_signal(Vec::<RequestSummary>::new());
            let (loading, _) = create_signal(false);
            let (error, _) = create_signal(None::<ApiError>);
            let message = create_rw_signal(MessageState::default());
            view! {
                <RequestsList
                    summaries=summaries.into()
                    loading=loading.into()
                    error=error.into()
                    on_select=Callback::new(|_| {})
                    on_edit=Callback::new(|_| {})
                    on_cancel=Callback::new(|_| {})
                    message=message
                />
            }
        });
        assert!(html.contains("表示できる申請がありません"));
    }

    #[test]
    fn requests_list_renders_rows() {
        let html = render_to_string(move || {
            let (summaries, _) = create_signal(vec![summary()]);
            let (loading, _) = create_signal(false);
            let (error, _) = create_signal(None::<ApiError>);
            let message = create_rw_signal(MessageState::default());
            view! {
                <RequestsList
                    summaries=summaries.into()
                    loading=loading.into()
                    error=error.into()
                    on_select=Callback::new(|_| {})
                    on_edit=Callback::new(|_| {})
                    on_cancel=Callback::new(|_| {})
                    message=message
                />
            }
        });
        assert!(html.contains("申請一覧"));
        assert!(html.contains("pending"));
        assert!(html.contains("休暇"));
    }

    #[test]
    fn requests_rows_include_keyboard_accessibility_attributes() {
        let html = render_to_string(move || {
            let (summaries, _) = create_signal(vec![summary()]);
            let (loading, _) = create_signal(false);
            let (error, _) = create_signal(None::<ApiError>);
            let message = create_rw_signal(MessageState::default());
            view! {
                <RequestsList
                    summaries=summaries.into()
                    loading=loading.into()
                    error=error.into()
                    on_select=Callback::new(|_| {})
                    on_edit=Callback::new(|_| {})
                    on_cancel=Callback::new(|_| {})
                    message=message
                />
            }
        });
        assert!(html.contains("tabindex=\"0\""));
        assert!(html.contains("role=\"button\""));
    }
}
