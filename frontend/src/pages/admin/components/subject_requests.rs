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

use crate::api::{
    ApiError, DataSubjectRequestResponse, DataSubjectRequestType, SubjectRequestListResponse,
    UserResponse,
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

fn lookup_username(users: &[UserResponse], user_id: &str) -> String {
    users
        .iter()
        .find(|u| u.id == user_id)
        .map(|u| u.username.clone())
        .unwrap_or_else(|| user_id.to_string())
}

fn subject_status_display(status: &str) -> String {
    match status {
        "pending" => "承認待ち".to_string(),
        "approved" => "承認済み".to_string(),
        "rejected" => "却下".to_string(),
        "cancelled" => "取消".to_string(),
        other => other.to_string(),
    }
}

fn subject_request_detail_rows(
    request: &DataSubjectRequestResponse,
    users: &[UserResponse],
) -> Vec<(&'static str, String)> {
    let mut rows: Vec<(&'static str, String)> = vec![
        ("ユーザー名", lookup_username(users, &request.user_id)),
        ("申請種別", type_label(&request.request_type).to_string()),
        (
            "ステータス",
            subject_status_display(&request.status).to_string(),
        ),
    ];
    if let Some(details) = &request.details {
        rows.push(("詳細", details.clone()));
    }
    rows.push(("申請日時", format_datetime(request.created_at)));
    rows.push(("更新日時", format_datetime(request.updated_at)));
    if let Some(by) = &request.approved_by {
        rows.push(("承認者", lookup_username(users, by)));
    }
    if let Some(at) = request.approved_at {
        rows.push(("承認日時", format_datetime(at)));
    }
    if let Some(by) = &request.rejected_by {
        rows.push(("却下者", lookup_username(users, by)));
    }
    if let Some(at) = request.rejected_at {
        rows.push(("却下日時", format_datetime(at)));
    }
    if let Some(at) = request.cancelled_at {
        rows.push(("取消日時", format_datetime(at)));
    }
    if let Some(comment) = &request.decision_comment {
        rows.push(("決定コメント", comment.clone()));
    }
    rows
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

fn bump_reload(reload: RwSignal<u32>) {
    reload.update(|value| *value = value.wrapping_add(1));
}

fn apply_status_filter_change(filter: &SubjectRequestFilterState, value: String) {
    filter.status_signal().set(value);
    filter.reset_page();
}

fn apply_type_filter_change(filter: &SubjectRequestFilterState, value: String) {
    filter.request_type_signal().set(value);
    filter.reset_page();
}

fn reset_page_and_reload(filter: &SubjectRequestFilterState, reload: RwSignal<u32>) {
    filter.reset_page();
    bump_reload(reload);
}

fn open_modal_state(
    modal_open: RwSignal<bool>,
    modal_request: RwSignal<Option<DataSubjectRequestResponse>>,
    modal_comment: RwSignal<String>,
    action_error: RwSignal<Option<ApiError>>,
    request: DataSubjectRequestResponse,
) {
    modal_request.set(Some(request));
    modal_comment.set(String::new());
    modal_open.set(true);
    action_error.set(None);
}

fn apply_action_result_state(
    result: Result<(), ApiError>,
    modal_open: RwSignal<bool>,
    modal_request: RwSignal<Option<DataSubjectRequestResponse>>,
    modal_comment: RwSignal<String>,
    action_error: RwSignal<Option<ApiError>>,
    reload: RwSignal<u32>,
) {
    let (should_close_modal, next_action_error, should_reload) = subject_action_feedback(result);
    action_error.set(next_action_error);
    if should_close_modal {
        modal_open.set(false);
        modal_request.set(None);
        modal_comment.set(String::new());
    }
    if should_reload {
        bump_reload(reload);
    }
}

fn apply_status_change_and_reload(
    filter: &SubjectRequestFilterState,
    reload: RwSignal<u32>,
    value: String,
) {
    apply_status_filter_change(filter, value);
    bump_reload(reload);
}

fn apply_type_change_and_reload(
    filter: &SubjectRequestFilterState,
    reload: RwSignal<u32>,
    value: String,
) {
    apply_type_filter_change(filter, value);
    bump_reload(reload);
}

fn perform_search_and_reload(filter: &SubjectRequestFilterState, reload: RwSignal<u32>) {
    reset_page_and_reload(filter, reload);
}

fn resolve_subject_action_payload(
    modal_request: Option<DataSubjectRequestResponse>,
    comment: &str,
    approve: bool,
    action_error: RwSignal<Option<ApiError>>,
) -> Option<SubjectRequestActionPayload> {
    match build_subject_action_payload(modal_request, comment, approve) {
        Ok(payload) => Some(payload),
        Err(err) => {
            action_error.set(Some(err));
            None
        }
    }
}

fn subject_request_error_message(error: Option<ApiError>) -> String {
    error.map(|err| err.to_string()).unwrap_or_default()
}

fn modal_error_message(action_error: Option<ApiError>) -> String {
    subject_request_error_message(action_error)
}

fn close_modal(modal_open: RwSignal<bool>) {
    modal_open.set(false);
}

fn update_modal_comment(modal_comment: RwSignal<String>, value: String) {
    modal_comment.set(value);
}

fn modal_action_disabled_for_request(
    action_pending: bool,
    request: Option<&DataSubjectRequestResponse>,
) -> bool {
    is_modal_action_disabled(action_pending, is_pending_request(request))
}

fn render_subject_request_rows(
    payload: Option<SubjectRequestListResponse>,
    open_modal: Callback<DataSubjectRequestResponse>,
) -> Vec<View> {
    payload
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
                    .into_view()
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
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

    let loading = resource.loading();
    let data = Signal::derive(move || resource.get().and_then(|result| result.ok()));
    let error = Signal::derive(move || resource.get().and_then(|result| result.err()));

    let action_pending = action.pending();

    create_effect(move |_| {
        if let Some(result) = action.value().get() {
            apply_action_result_state(
                result,
                modal_open,
                modal_request,
                modal_comment,
                action_error,
                reload,
            );
        }
    });

    let on_status_change = move |value: String| {
        apply_status_change_and_reload(&filter, reload, value);
    };

    let on_type_change = move |value: String| {
        apply_type_change_and_reload(&filter, reload, value);
    };

    let on_search = move |_| {
        perform_search_and_reload(&filter, reload);
    };

    let open_modal = Callback::new(move |request: DataSubjectRequestResponse| {
        open_modal_state(
            modal_open,
            modal_request,
            modal_comment,
            action_error,
            request,
        );
    });

    let on_action = move |approve: bool| {
        let Some(payload) = resolve_subject_action_payload(
            modal_request.get(),
            &modal_comment.get(),
            approve,
            action_error,
        ) else {
            return;
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
                <ErrorMessage message={subject_request_error_message(error.get())} />
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
                                {move || render_subject_request_rows(data.get(), open_modal)}
                        </Show>
                    </tbody>
                </table>
            </div>
            <Show when=move || modal_open.get()>
                <div class="fixed inset-0 bg-overlay-backdrop flex items-center justify-center z-50">
                    <div class="bg-surface-elevated rounded-lg shadow-lg w-full max-w-lg p-6">
                        <h3 class="text-lg font-medium text-fg mb-2">{"本人対応申請の詳細"}</h3>
                        <div class="overflow-y-auto max-h-64 divide-y divide-border-subtle">
                            {move || {
                                let user_list =
                                    users.get().and_then(|r| r.ok()).unwrap_or_default();
                                modal_request
                                    .get()
                                    .map(|r| {
                                        subject_request_detail_rows(&r, &user_list)
                                            .into_iter()
                                            .map(|(label, value)| {
                                                view! {
                                                    <div class="flex gap-3 py-1.5 text-sm">
                                                        <span class="text-fg-muted font-medium w-28 shrink-0">
                                                            {label}
                                                        </span>
                                                        <span class="text-fg break-words min-w-0">
                                                            {value}
                                                        </span>
                                                    </div>
                                                }
                                            })
                                            .collect::<Vec<_>>()
                                    })
                                    .unwrap_or_default()
                            }}
                        </div>
                        <div class="mt-3">
                            <label class="block text-sm font-medium text-fg-muted">{"コメント"}</label>
                            <textarea
                                class="w-full border border-form-control-border bg-form-control-bg text-form-control-text rounded px-2 py-1"
                                on:input=move |ev| {
                                    update_modal_comment(modal_comment, event_target_value(&ev))
                                }
                            ></textarea>
                        </div>
                        <Show when=move || action_error.get().is_some()>
                            <ErrorMessage
                                message={modal_error_message(action_error.get())}
                            />
                        </Show>
                        <div class="mt-4 flex justify-end space-x-2">
                            <button
                                class="px-3 py-1 rounded border border-border text-fg hover:bg-action-ghost-bg-hover"
                                on:click=move |_| close_modal(modal_open)
                            >
                                {"閉じる"}
                            </button>
                            <button
                                class="px-3 py-1 rounded bg-action-danger-bg text-action-danger-text disabled:opacity-50"
                                disabled={move || {
                                    modal_action_disabled_for_request(
                                        action_pending.get(),
                                        modal_request.get().as_ref(),
                                    )
                                }}
                                on:click=move |_| on_action(false)
                            >
                                {"却下"}
                            </button>
                            <button
                                class="px-3 py-1 rounded bg-action-primary-bg text-action-primary-text disabled:opacity-50"
                                disabled={move || {
                                    modal_action_disabled_for_request(
                                        action_pending.get(),
                                        modal_request.get().as_ref(),
                                    )
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
    use crate::test_support::ssr::{render_to_string, with_runtime};
    use chrono::Utc;

    fn sample_request() -> DataSubjectRequestResponse {
        DataSubjectRequestResponse {
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
        }
    }

    fn render_with_items(items: Vec<DataSubjectRequestResponse>) -> String {
        render_to_string(move || {
            let users = Resource::new(|| true, |_| async move { Ok(Vec::new()) });
            let filter = SubjectRequestFilterState::new();
            let total = items.len() as i64;
            let resource = Resource::new(
                move || (true, filter.snapshot(), 0u32),
                |_| async move {
                    Ok(SubjectRequestListResponse {
                        page: 1,
                        per_page: 20,
                        total: 0,
                        items: Vec::new(),
                    })
                },
            );
            resource.set(Ok(SubjectRequestListResponse {
                page: 1,
                per_page: 20,
                total,
                items,
            }));
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
        let request = sample_request();

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
        // UTC 固定のため値は安定し、フォーマットは固定
        let formatted = format_datetime(dt);
        assert_eq!(formatted.len(), 19, "YYYY/MM/DD HH:MM:SS = 19 chars");
        assert_eq!(&formatted[4..5], "/");
        assert_eq!(&formatted[7..8], "/");
        assert_eq!(&formatted[10..11], " ");
        assert_eq!(&formatted[13..14], ":");
        assert_eq!(&formatted[16..17], ":");
    }

    #[test]
    fn helper_modal_pending_and_disable_logic() {
        let pending_request = sample_request();
        let approved_request = DataSubjectRequestResponse {
            status: "approved".into(),
            ..pending_request.clone()
        };

        assert!(is_pending_request(Some(&pending_request)));
        assert!(!is_pending_request(Some(&approved_request)));
        assert!(!is_pending_request(None));

        assert!(is_modal_action_disabled(true, true));
        assert!(is_modal_action_disabled(true, false));
        assert!(is_modal_action_disabled(false, false));
        assert!(!is_modal_action_disabled(false, true));
    }

    #[test]
    fn helper_subject_request_detail_rows_basic_fields() {
        let request = sample_request();
        let rows = subject_request_detail_rows(&request, &[]);
        let labels: Vec<&str> = rows.iter().map(|(l, _)| *l).collect();
        assert!(!labels.contains(&"ID"), "ID は非表示");
        assert!(labels.contains(&"ユーザー名"));
        assert!(labels.contains(&"申請種別"));
        assert!(labels.contains(&"ステータス"));
        assert!(labels.contains(&"申請日時"));
        assert!(labels.contains(&"更新日時"));
        let status_row = rows.iter().find(|(l, _)| *l == "ステータス").unwrap();
        assert_eq!(status_row.1, "承認待ち");
        let type_row = rows.iter().find(|(l, _)| *l == "申請種別").unwrap();
        assert_eq!(type_row.1, "開示");
        // users=[] なのでフォールバックとして user_id が表示される
        let user_row = rows.iter().find(|(l, _)| *l == "ユーザー名").unwrap();
        assert_eq!(user_row.1, "u1");
        // 申請日時は YYYY/MM/DD HH:MM:SS 形式
        let created_row = rows.iter().find(|(l, _)| *l == "申請日時").unwrap();
        assert_eq!(created_row.1.len(), 19);
        assert_eq!(&created_row.1[4..5], "/");
        assert_eq!(&created_row.1[7..8], "/");
        assert_eq!(&created_row.1[16..17], ":");
    }

    #[test]
    fn helper_subject_request_detail_rows_resolves_username() {
        use crate::api::UserResponse;
        let users = vec![UserResponse {
            id: "u1".into(),
            username: "hanako".into(),
            full_name: "Hanako".into(),
            role: "employee".into(),
            is_system_admin: false,
            mfa_enabled: false,
            is_locked: false,
            locked_until: None,
            failed_login_attempts: 0,
            password_expiry_warning_days: None,
            department_id: None,
        }];
        let rows = subject_request_detail_rows(&sample_request(), &users);
        let user_row = rows.iter().find(|(l, _)| *l == "ユーザー名").unwrap();
        assert_eq!(user_row.1, "hanako");
    }

    #[test]
    fn helper_subject_request_detail_rows_optional_fields() {
        let request = DataSubjectRequestResponse {
            approved_by: Some("u1".to_string()),
            rejected_by: Some("u2".to_string()),
            details: Some("修正希望".to_string()),
            decision_comment: Some("確認しました".to_string()),
            ..sample_request()
        };
        let users = vec![
            UserResponse {
                id: "u1".into(),
                username: "alice".into(),
                full_name: "Alice".into(),
                role: "employee".into(),
                is_system_admin: false,
                mfa_enabled: false,
                is_locked: false,
                locked_until: None,
                failed_login_attempts: 0,
                password_expiry_warning_days: None,
                department_id: None,
            },
            UserResponse {
                id: "u2".into(),
                username: "bob".into(),
                full_name: "Bob".into(),
                role: "manager".into(),
                is_system_admin: false,
                mfa_enabled: false,
                is_locked: false,
                locked_until: None,
                failed_login_attempts: 0,
                password_expiry_warning_days: None,
                department_id: None,
            },
        ];
        let rows = subject_request_detail_rows(&request, &users);
        let labels: Vec<&str> = rows.iter().map(|(l, _)| *l).collect();
        assert!(labels.contains(&"詳細"));
        assert!(labels.contains(&"決定コメント"));
        assert!(labels.contains(&"承認者"));
        assert!(labels.contains(&"却下者"));
        let details_row = rows.iter().find(|(l, _)| *l == "詳細").unwrap();
        assert_eq!(details_row.1, "修正希望");
        let approved_row = rows.iter().find(|(l, _)| *l == "承認者").unwrap();
        let rejected_row = rows.iter().find(|(l, _)| *l == "却下者").unwrap();
        assert_eq!(approved_row.1, "alice");
        assert_eq!(rejected_row.1, "bob");
    }

    #[test]
    fn helper_subject_request_detail_rows_excludes_none_optional_fields() {
        let request = sample_request();
        let rows = subject_request_detail_rows(&request, &[]);
        let labels: Vec<&str> = rows.iter().map(|(l, _)| *l).collect();
        assert!(!labels.contains(&"詳細"));
        assert!(!labels.contains(&"承認者"));
        assert!(!labels.contains(&"却下者"));
        assert!(!labels.contains(&"決定コメント"));
    }

    #[test]
    fn helper_subject_status_display_translates_statuses() {
        assert_eq!(subject_status_display("pending"), "承認待ち".to_string());
        assert_eq!(subject_status_display("approved"), "承認済み".to_string());
        assert_eq!(subject_status_display("rejected"), "却下".to_string());
        assert_eq!(subject_status_display("cancelled"), "取消".to_string());
        assert_eq!(subject_status_display("unknown"), "unknown".to_string());
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

    #[test]
    fn helper_filter_reload_and_modal_state_updates_cover_paths() {
        with_runtime(|| {
            let filter = SubjectRequestFilterState::new();
            let reload = create_rw_signal(0u32);
            let modal_open = create_rw_signal(false);
            let modal_request = create_rw_signal(None::<DataSubjectRequestResponse>);
            let modal_comment = create_rw_signal("preset".to_string());
            let action_error = create_rw_signal(Some(ApiError::unknown("prev-error")));

            apply_status_filter_change(&filter, "pending".to_string());
            assert_eq!(filter.status_signal().get(), "pending");
            assert_eq!(filter.snapshot().page, 1);

            apply_type_filter_change(&filter, "access".to_string());
            assert_eq!(filter.request_type_signal().get(), "access");
            assert_eq!(filter.snapshot().page, 1);

            bump_reload(reload);
            assert_eq!(reload.get(), 1);

            reset_page_and_reload(&filter, reload);
            assert_eq!(reload.get(), 2);
            assert_eq!(filter.snapshot().page, 1);

            open_modal_state(
                modal_open,
                modal_request,
                modal_comment,
                action_error,
                sample_request(),
            );
            assert!(modal_open.get());
            assert!(modal_request.get().is_some());
            assert_eq!(modal_comment.get(), "");
            assert!(action_error.get().is_none());
        });
    }

    #[test]
    fn helper_action_state_and_payload_resolution_cover_paths() {
        with_runtime(|| {
            let modal_open = create_rw_signal(true);
            let modal_request = create_rw_signal(Some(sample_request()));
            let modal_comment = create_rw_signal("memo".to_string());
            let action_error = create_rw_signal(None::<ApiError>);
            let reload = create_rw_signal(0u32);

            apply_action_result_state(
                Ok(()),
                modal_open,
                modal_request,
                modal_comment,
                action_error,
                reload,
            );
            assert!(!modal_open.get());
            assert!(modal_request.get().is_none());
            assert_eq!(modal_comment.get(), "");
            assert!(action_error.get().is_none());
            assert_eq!(reload.get(), 1);

            modal_open.set(true);
            modal_request.set(Some(sample_request()));
            modal_comment.set("memo".to_string());
            apply_action_result_state(
                Err(ApiError::unknown("dispatch failed")),
                modal_open,
                modal_request,
                modal_comment,
                action_error,
                reload,
            );
            assert!(modal_open.get());
            assert!(modal_request.get().is_some());
            assert_eq!(modal_comment.get(), "memo");
            assert_eq!(
                action_error.get().as_ref().expect("error").error,
                "dispatch failed"
            );
            assert_eq!(reload.get(), 1);

            let payload = resolve_subject_action_payload(
                Some(sample_request()),
                "approved",
                true,
                action_error,
            )
            .expect("payload");
            assert_eq!(payload.id, "sr-1");
            assert_eq!(payload.comment, "approved");
            assert!(payload.approve);

            let invalid =
                resolve_subject_action_payload(Some(sample_request()), "   ", false, action_error);
            assert!(invalid.is_none());
            assert_eq!(
                action_error.get().as_ref().expect("validation").code,
                "VALIDATION_ERROR"
            );
        });
    }

    #[test]
    fn helper_filter_reload_render_and_modal_helpers_cover_paths() {
        with_runtime(|| {
            let filter = SubjectRequestFilterState::new();
            let reload = create_rw_signal(0u32);
            apply_status_change_and_reload(&filter, reload, "pending".to_string());
            assert_eq!(filter.status_signal().get(), "pending");
            assert_eq!(reload.get(), 1);

            apply_type_change_and_reload(&filter, reload, "delete".to_string());
            assert_eq!(filter.request_type_signal().get(), "delete");
            assert_eq!(reload.get(), 2);

            perform_search_and_reload(&filter, reload);
            assert_eq!(reload.get(), 3);
            assert_eq!(filter.snapshot().page, 1);

            let rows = render_subject_request_rows(
                Some(SubjectRequestListResponse {
                    page: 1,
                    per_page: 20,
                    total: 1,
                    items: vec![sample_request()],
                }),
                Callback::new(|_: DataSubjectRequestResponse| {}),
            );
            assert_eq!(rows.len(), 1);
            let html = rows[0].clone().render_to_string().to_string();
            assert!(html.contains("開示"));
            assert!(html.contains("u1"));
            assert!(html.contains("pending"));
            assert!(html.contains("sr-1"));
            assert!(render_subject_request_rows(
                None,
                Callback::new(|_: DataSubjectRequestResponse| {})
            )
            .is_empty());

            assert_eq!(subject_request_error_message(None), "");
            assert_eq!(modal_error_message(None), "");
            assert_eq!(
                subject_request_error_message(Some(ApiError::unknown("err"))),
                "err"
            );
            assert_eq!(
                modal_error_message(Some(ApiError::unknown("modal err"))),
                "modal err"
            );

            let modal_comment = create_rw_signal(String::new());
            update_modal_comment(modal_comment, "note".to_string());
            assert_eq!(modal_comment.get(), "note");

            let modal_open = create_rw_signal(true);
            close_modal(modal_open);
            assert!(!modal_open.get());

            let pending_request = sample_request();
            let approved_request = DataSubjectRequestResponse {
                status: "approved".to_string(),
                ..pending_request.clone()
            };
            assert!(modal_action_disabled_for_request(
                true,
                Some(&pending_request)
            ));
            assert!(!modal_action_disabled_for_request(
                false,
                Some(&pending_request)
            ));
            assert!(modal_action_disabled_for_request(
                false,
                Some(&approved_request)
            ));
            assert!(modal_action_disabled_for_request(false, None));
        });
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
    value
        .with_timezone(&chrono::Utc)
        .format("%Y/%m/%d %H:%M:%S")
        .to_string()
}
