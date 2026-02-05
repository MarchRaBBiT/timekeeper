use crate::{
    components::layout::{ErrorMessage, LoadingSpinner},
    pages::admin::{
        components::user_select::{AdminUserSelect, UsersResource},
        utils::SubjectRequestFilterState,
        view_model::SubjectRequestActionPayload,
    },
};
use chrono::DateTime;
use leptos::*;
use serde_json::to_string_pretty;

use crate::api::{
    ApiError, DataSubjectRequestResponse, DataSubjectRequestType, SubjectRequestListResponse,
};

fn build_subject_action_payload(
    modal_request: Option<DataSubjectRequestResponse>,
    comment: &str,
    approve: bool,
) -> Result<SubjectRequestActionPayload, ApiError> {
    if comment.trim().is_empty() {
        return Err(ApiError::validation("コメントを入力してください。"));
    }
    let request =
        modal_request.ok_or_else(|| ApiError::validation("申請情報を取得できませんでした。"))?;
    Ok(SubjectRequestActionPayload {
        id: request.id,
        comment: comment.to_string(),
        approve,
    })
}

fn modal_detail_json(request: Option<&DataSubjectRequestResponse>) -> String {
    request
        .and_then(|request| to_string_pretty(request).ok())
        .unwrap_or_default()
}

fn is_pending_request(request: Option<&DataSubjectRequestResponse>) -> bool {
    request
        .map(|request| request.status == "pending")
        .unwrap_or(false)
}

fn is_modal_action_disabled(action_pending: bool, modal_pending: bool) -> bool {
    action_pending || !modal_pending
}

fn subject_action_feedback(result: Result<(), ApiError>) -> (bool, Option<ApiError>, bool) {
    match result {
        Ok(_) => (true, None, true),
        Err(err) => (false, Some(err), false),
    }
}

#[component]
pub fn AdminSubjectRequestsSection(
    users: UsersResource,
    filter: SubjectRequestFilterState,
    resource: Resource<
        (
            bool,
            crate::pages::admin::utils::SubjectRequestFilterSnapshot,
            u32,
        ),
        Result<SubjectRequestListResponse, ApiError>,
    >,
    action: Action<SubjectRequestActionPayload, Result<(), ApiError>>,
    action_error: RwSignal<Option<ApiError>>,
    reload: RwSignal<u32>,
) -> impl IntoView {
    let modal_open = create_rw_signal(false);
    let modal_request = create_rw_signal(None::<DataSubjectRequestResponse>);
    let modal_comment = create_rw_signal(String::new());

    let modal_detail = Signal::derive(move || modal_detail_json(modal_request.get().as_ref()));
    let modal_pending = Signal::derive(move || is_pending_request(modal_request.get().as_ref()));

    let loading = resource.loading();
    let data = Signal::derive(move || resource.get().and_then(|result| result.ok()));
    let error = Signal::derive(move || resource.get().and_then(|result| result.err()));

    let action_pending = action.pending();

    create_effect(move |_| {
        if let Some(result) = action.value().get() {
            let (should_close_modal, next_action_error, should_reload) =
                subject_action_feedback(result);
            action_error.set(next_action_error);
            if should_close_modal {
                modal_open.set(false);
                modal_request.set(None);
                modal_comment.set(String::new());
            }
            if should_reload {
                reload.update(|value| *value = value.wrapping_add(1));
            }
        }
    });

    let trigger_reload = move || reload.update(|value| *value = value.wrapping_add(1));

    let on_status_change = move |value: String| {
        filter.status_signal().set(value);
        filter.reset_page();
        trigger_reload();
    };

    let on_type_change = move |value: String| {
        filter.request_type_signal().set(value);
        filter.reset_page();
        trigger_reload();
    };

    let on_search = move |_| {
        filter.reset_page();
        trigger_reload();
    };

    let open_modal = Callback::new(move |request: DataSubjectRequestResponse| {
        modal_request.set(Some(request));
        modal_comment.set(String::new());
        modal_open.set(true);
        action_error.set(None);
    });

    let on_action = move |approve: bool| {
        let payload = match build_subject_action_payload(
            modal_request.get(),
            &modal_comment.get(),
            approve,
        ) {
            Ok(payload) => payload,
            Err(err) => {
                action_error.set(Some(err));
                return;
            }
        };
        action.dispatch(payload);
    };

    view! {
        <div class="bg-surface-elevated shadow rounded-lg p-6 space-y-4">
            <h3 class="text-lg font-medium text-fg">{"本人対応申請"}</h3>
            <div class="flex flex-col gap-3 lg:flex-row lg:flex-wrap lg:items-end">
                <select
                    class="w-full lg:w-auto border border-form-control-border bg-form-control-bg text-form-control-text rounded-md px-2 py-1"
                    on:change=move |ev| on_status_change(event_target_value(&ev))
                >
                    <option value="">{ "すべて" }</option>
                    <option value="pending">{ "承認待ち" }</option>
                    <option value="approved">{ "承認済み" }</option>
                    <option value="rejected">{ "却下" }</option>
                    <option value="cancelled">{ "取消" }</option>
                </select>
                <select
                    class="w-full lg:w-auto border border-form-control-border bg-form-control-bg text-form-control-text rounded-md px-2 py-1"
                    on:change=move |ev| on_type_change(event_target_value(&ev))
                >
                    <option value="">{ "請求種別" }</option>
                    <option value="access">{ "開示" }</option>
                    <option value="rectify">{ "訂正" }</option>
                    <option value="delete">{ "削除" }</option>
                    <option value="stop">{ "停止" }</option>
                </select>
                <div class="w-full lg:min-w-[220px] lg:flex-1">
                    <AdminUserSelect
                        users=users
                        selected=filter.user_id_signal()
                        label=Some("請求種別".into())
                        placeholder="全ユーザー".into()
                    />
                </div>
                <button
                    class="w-full lg:w-auto px-3 py-1 bg-action-primary-bg text-action-primary-text rounded"
                    disabled={move || loading.get()}
                    on:click=on_search
                >
                    <span class="inline-flex items-center gap-2">
                        <Show when=move || loading.get()>
                            <span class="h-4 w-4 animate-spin rounded-full border-2 border-action-primary-text/70 border-t-transparent"></span>
                        </Show>
                        {move || if loading.get() { "検索中..." } else { "検索" }}
                    </span>
                </button>
            </div>
            <Show when=move || error.get().is_some()>
                <ErrorMessage message={error.get().map(|err| err.to_string()).unwrap_or_default()} />
            </Show>
            <Show when=move || loading.get()>
                <div class="flex items-center gap-2 text-sm text-fg-muted">
                    <LoadingSpinner />
                    <span>{"本人対応申請を読み込み中..."}</span>
                </div>
            </Show>
            <div class="overflow-x-auto">
                <table class="min-w-full divide-y divide-border">
                    <thead class="bg-surface-muted">
                        <tr>
                            <th class="px-6 py-3 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">{"種別"}</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">{"ユーザー"}</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">{"ステータス"}</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">{"申請日"}</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody class="bg-surface-elevated divide-y divide-border">
                        <Show when=move || data.get().is_some()>
                            {move || {
                                data.get()
                                    .map(|payload| {
                                        payload
                                            .items
                                            .into_iter()
                                            .map(|item| {
                                                let status_label = item.status.clone();
                                                let created_label = format_datetime(item.created_at);
                                                let type_label = type_label(&item.request_type);
                                                let id = item.id.clone();
                                                let open = {
                                                    let item = item.clone();
                                                    let open_modal = open_modal.clone();
                                                    move |_| open_modal.call(item.clone())
                                                };
                                                view! {
                                                    <tr>
                                                        <td class="px-6 py-4 whitespace-nowrap text-sm text-fg">{type_label}</td>
                                                        <td class="px-6 py-4 whitespace-nowrap text-sm text-fg">{item.user_id}</td>
                                                        <td class="px-6 py-4 whitespace-nowrap">
                                                            <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-status-neutral-bg text-status-neutral-text">
                                                                {status_label}
                                                            </span>
                                                        </td>
                                                        <td class="px-6 py-4 whitespace-nowrap text-sm text-fg">{created_label}</td>
                                                        <td class="px-6 py-4 whitespace-nowrap text-right text-sm">
                                                            <button class="text-link hover:text-link-hover" on:click=open>{"詳細"}</button>
                                                            <span class="sr-only">{id}</span>
                                                        </td>
                                                    </tr>
                                                }
                                            })
                                            .collect::<Vec<_>>()
                                    })
                                    .unwrap_or_default()
                            }}
                        </Show>
                    </tbody>
                </table>
            </div>
            <Show when=move || modal_open.get()>
                <div class="fixed inset-0 bg-overlay-backdrop flex items-center justify-center z-50">
                    <div class="bg-surface-elevated rounded-lg shadow-lg w-full max-w-lg p-6">
                        <h3 class="text-lg font-medium text-fg mb-2">{"本人対応申請の詳細"}</h3>
                        <pre class="text-xs bg-surface-muted text-fg p-2 rounded overflow-auto max-h-64 whitespace-pre-wrap">{move || modal_detail.get()}</pre>
                        <div class="mt-3">
                            <label class="block text-sm font-medium text-fg-muted">{"コメント"}</label>
                            <textarea
                                class="w-full border border-form-control-border bg-form-control-bg text-form-control-text rounded px-2 py-1"
                                on:input=move |ev| modal_comment.set(event_target_value(&ev))
                            ></textarea>
                        </div>
                        <Show when=move || action_error.get().is_some()>
                            <ErrorMessage
                                message={action_error
                                    .get()
                                    .map(|err| err.to_string())
                                    .unwrap_or_default()}
                            />
                        </Show>
                        <div class="mt-4 flex justify-end space-x-2">
                            <button class="px-3 py-1 rounded border border-border text-fg hover:bg-action-ghost-bg-hover" on:click=move |_| modal_open.set(false)>{"閉じる"}</button>
                            <button
                                class="px-3 py-1 rounded bg-action-danger-bg text-action-danger-text disabled:opacity-50"
                                disabled={move || {
                                    is_modal_action_disabled(action_pending.get(), modal_pending.get())
                                }}
                                on:click=move |_| on_action(false)
                            >
                                {"却下"}
                            </button>
                            <button
                                class="px-3 py-1 rounded bg-action-primary-bg text-action-primary-text disabled:opacity-50"
                                disabled={move || {
                                    is_modal_action_disabled(action_pending.get(), modal_pending.get())
                                }}
                                on:click=move |_| on_action(true)
                            >
                                {"承認"}
                            </button>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::test_support::ssr::render_to_string;
    use chrono::Utc;

    fn render_with_items(items: Vec<DataSubjectRequestResponse>) -> String {
        render_to_string(move || {
            let users = Resource::new(|| true, |_| async move { Ok(Vec::new()) });
            let filter = SubjectRequestFilterState::new();
            let resource = Resource::new(
                move || (true, filter.snapshot(), 0u32),
                move |_| {
                    let items = items.clone();
                    async move {
                        Ok(SubjectRequestListResponse {
                            page: 1,
                            per_page: 20,
                            total: items.len() as i64,
                            items,
                        })
                    }
                },
            );
            let action = create_action(|_: &SubjectRequestActionPayload| async move { Ok(()) });
            let action_error = create_rw_signal(None::<ApiError>);
            let reload = create_rw_signal(0u32);
            view! {
                <AdminSubjectRequestsSection
                    users=users
                    filter=filter
                    resource=resource
                    action=action
                    action_error=action_error
                    reload=reload
                />
            }
        })
    }

    #[test]
    fn subject_requests_renders_empty() {
        let html = render_with_items(Vec::new());
        assert!(html.contains("本人対応申請"));
    }

    #[test]
    fn subject_requests_renders_row() {
        let html = render_with_items(vec![DataSubjectRequestResponse {
            id: "sr-1".into(),
            user_id: "u1".into(),
            request_type: DataSubjectRequestType::Access,
            status: "pending".into(),
            details: None,
            approved_by: None,
            approved_at: None,
            rejected_by: None,
            rejected_at: None,
            cancelled_at: None,
            decision_comment: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }]);
        assert!(html.contains("開示"));
        assert!(html.contains("pending"));
    }

    #[test]
    fn helper_build_subject_action_payload_validates_inputs() {
        let request = DataSubjectRequestResponse {
            id: "sr-1".into(),
            user_id: "u1".into(),
            request_type: DataSubjectRequestType::Access,
            status: "pending".into(),
            details: None,
            approved_by: None,
            approved_at: None,
            rejected_by: None,
            rejected_at: None,
            cancelled_at: None,
            decision_comment: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(build_subject_action_payload(None, "comment", true).is_err());
        assert!(build_subject_action_payload(Some(request.clone()), " ", true).is_err());
        let payload = build_subject_action_payload(Some(request), "ok", false).expect("payload");
        assert_eq!(payload.id, "sr-1");
        assert_eq!(payload.comment, "ok");
        assert!(!payload.approve);
    }

    #[test]
    fn helper_type_and_datetime_formatting() {
        assert_eq!(type_label(&DataSubjectRequestType::Access), "開示");
        assert_eq!(type_label(&DataSubjectRequestType::Rectify), "訂正");
        assert_eq!(type_label(&DataSubjectRequestType::Delete), "削除");
        assert_eq!(type_label(&DataSubjectRequestType::Stop), "停止");

        let dt = DateTime::parse_from_rfc3339("2026-01-16T12:34:56Z")
            .expect("valid datetime")
            .with_timezone(&chrono::Utc);
        assert_eq!(format_datetime(dt), "2026-01-16 12:34");
    }

    #[test]
    fn helper_modal_pending_and_disable_logic() {
        let pending_request = DataSubjectRequestResponse {
            id: "sr-1".into(),
            user_id: "u1".into(),
            request_type: DataSubjectRequestType::Access,
            status: "pending".into(),
            details: None,
            approved_by: None,
            approved_at: None,
            rejected_by: None,
            rejected_at: None,
            cancelled_at: None,
            decision_comment: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let approved_request = DataSubjectRequestResponse {
            status: "approved".into(),
            ..pending_request.clone()
        };

        assert!(is_pending_request(Some(&pending_request)));
        assert!(!is_pending_request(Some(&approved_request)));
        assert!(!is_pending_request(None));

        assert!(is_modal_action_disabled(true, true));
        assert!(is_modal_action_disabled(false, false));
        assert!(!is_modal_action_disabled(false, true));
    }

    #[test]
    fn helper_modal_detail_json_handles_none_and_some() {
        assert_eq!(modal_detail_json(None), "");

        let request = DataSubjectRequestResponse {
            id: "sr-1".into(),
            user_id: "u1".into(),
            request_type: DataSubjectRequestType::Access,
            status: "pending".into(),
            details: None,
            approved_by: None,
            approved_at: None,
            rejected_by: None,
            rejected_at: None,
            cancelled_at: None,
            decision_comment: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let rendered = modal_detail_json(Some(&request));
        assert!(rendered.contains("\"id\": \"sr-1\""));
        assert!(rendered.contains("\"status\": \"pending\""));
    }

    #[test]
    fn helper_subject_action_feedback_maps_success_and_error() {
        let (ok_close, ok_error, ok_reload) = subject_action_feedback(Ok(()));
        assert!(ok_close);
        assert!(ok_error.is_none());
        assert!(ok_reload);

        let (err_close, err_error, err_reload) =
            subject_action_feedback(Err(ApiError::unknown("action failed")));
        assert!(!err_close);
        assert_eq!(err_error.expect("error").error, "action failed");
        assert!(!err_reload);
    }

    #[test]
    fn helper_build_subject_action_payload_preserves_comment_and_approve_flag() {
        let request = DataSubjectRequestResponse {
            id: "sr-approve".into(),
            user_id: "u-approve".into(),
            request_type: DataSubjectRequestType::Rectify,
            status: "pending".into(),
            details: None,
            approved_by: None,
            approved_at: None,
            rejected_by: None,
            rejected_at: None,
            cancelled_at: None,
            decision_comment: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let payload =
            build_subject_action_payload(Some(request), "  keep-spaces  ", true).expect("payload");
        assert_eq!(payload.id, "sr-approve");
        assert_eq!(payload.comment, "  keep-spaces  ");
        assert!(payload.approve);
    }
}

fn type_label(request_type: &DataSubjectRequestType) -> &'static str {
    match request_type {
        DataSubjectRequestType::Access => "開示",
        DataSubjectRequestType::Rectify => "訂正",
        DataSubjectRequestType::Delete => "削除",
        DataSubjectRequestType::Stop => "停止",
    }
}

fn format_datetime(value: DateTime<chrono::Utc>) -> String {
    value.format("%Y-%m-%d %H:%M").to_string()
}
