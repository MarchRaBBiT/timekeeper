use super::view_model::{use_audit_log_view_model, AuditLogFilters, AUDIT_EVENT_TYPES};
use crate::api::{ApiError, AuditLog, AuditLogListResponse};
use crate::components::common::{Button, ButtonVariant};
use crate::components::empty_state::EmptyState;
use crate::components::layout::Layout;
use crate::state::auth::use_auth;
use leptos::*;

fn update_filter_value(filters: &mut AuditLogFilters, field: &str, value: String) {
    match field {
        "from" => filters.from = value,
        "to" => filters.to = value,
        "actor_id" => filters.actor_id = value,
        "event_type" => filters.event_type = value,
        "result" => filters.result = value,
        _ => {}
    }
}

fn compute_total_pages(total: i64, per_page: i64) -> i64 {
    if total <= 0 || per_page <= 0 {
        1
    } else {
        ((total + per_page - 1) / per_page).max(1)
    }
}

fn can_go_next(page: i64, total: i64, per_page: i64) -> bool {
    page < compute_total_pages(total, per_page)
}

fn metadata_preview(value: &serde_json::Value, max_len: usize) -> String {
    let rendered = value.to_string();
    let rendered_len = rendered.chars().count();
    if rendered_len > max_len {
        let truncated: String = rendered.chars().take(max_len).collect();
        format!("{truncated}...")
    } else {
        rendered
    }
}

fn result_badge_class(result: &str) -> &'static str {
    if result == "success" {
        "px-2 inline-flex text-xs leading-5 font-semibold rounded-full bg-status-success-bg text-status-success-text"
    } else {
        "px-2 inline-flex text-xs leading-5 font-semibold rounded-full bg-status-error-bg text-status-error-text"
    }
}

fn page_summary_label(total: i64, per_page: i64) -> String {
    format!(" / {}", compute_total_pages(total, per_page))
}

fn apply_filter_change(filters: &mut AuditLogFilters, page: &mut i64, field: &str, value: String) {
    update_filter_value(filters, field, value);
    *page = 1;
}

fn clear_filters_and_reset_page(filters: &mut AuditLogFilters, page: &mut i64) {
    *filters = AuditLogFilters::default();
    *page = 1;
}

fn previous_page(page: i64) -> i64 {
    (page - 1).max(1)
}

fn next_page(page: i64) -> i64 {
    page + 1
}

fn actor_label(actor_id: Option<String>) -> String {
    actor_id.unwrap_or_else(|| "-".to_string())
}

fn target_label(target_type: Option<String>, target_id: Option<String>) -> String {
    format!(
        "{} {}",
        target_type.unwrap_or_default(),
        target_id.unwrap_or_default()
    )
}

fn metadata_button_payload(
    metadata: Option<serde_json::Value>,
    max_len: usize,
) -> Option<(String, serde_json::Value)> {
    metadata.map(|value| (metadata_preview(&value, max_len), value))
}

fn metadata_modal_pretty(selected: Option<serde_json::Value>) -> String {
    selected
        .and_then(|value| serde_json::to_string_pretty(&value).ok())
        .unwrap_or_default()
}

fn clear_selected_metadata(selected: &mut Option<serde_json::Value>) {
    *selected = None;
}

fn page_summary_from_logs(logs: Option<Result<AuditLogListResponse, ApiError>>) -> String {
    logs.map(|res| match res {
        Ok(response) => page_summary_label(response.total, response.per_page),
        Err(_) => String::new(),
    })
    .unwrap_or_default()
}

fn is_next_page_disabled(page: i64, logs: Option<Result<AuditLogListResponse, ApiError>>) -> bool {
    logs.map(|res| match res {
        Ok(response) => !can_go_next(page, response.total, response.per_page),
        Err(_) => true,
    })
    .unwrap_or(true)
}

fn render_log_row(log: AuditLog, selected_metadata: RwSignal<Option<serde_json::Value>>) -> View {
    let actor = actor_label(log.actor_id);
    let target = target_label(log.target_type, log.target_id);
    let result_text = log.result;
    let result_class = result_badge_class(&result_text);
    let error_code = log.error_code;
    let metadata_payload = metadata_button_payload(log.metadata, 50);

    view! {
        <tr>
            <td class="px-6 py-4 whitespace-nowrap text-sm text-fg-muted">
                {log.occurred_at.format("%Y-%m-%d %H:%M:%S").to_string()}
            </td>
            <td class="px-6 py-4 whitespace-nowrap text-sm text-fg">
                {actor}
                <span class="text-xs text-fg-muted block">{log.actor_type}</span>
            </td>
            <td class="px-6 py-4 whitespace-nowrap text-sm text-fg">
                {log.event_type}
            </td>
            <td class="px-6 py-4 whitespace-nowrap text-sm text-fg-muted">
                {target}
            </td>
             <td class="px-6 py-4 whitespace-nowrap text-sm">
                <span class=result_class>
                    {result_text}
                </span>
                {error_code.map(|code| view! { <span class="block text-xs text-status-error-text">{code}</span> })}
            </td>
             <td class="px-6 py-4 whitespace-nowrap text-sm text-fg-muted">
                 {metadata_payload.map(|(display, metadata)| view! {
                    <button
                        class="text-link hover:text-link-hover hover:underline text-left font-mono text-xs"
                        on:click=move |_| selected_metadata.set(Some(metadata.clone()))
                    >
                        {display}
                    </button>
                 })}
            </td>
        </tr>
    }
    .into_view()
}

fn render_logs_content(
    result: Result<AuditLogListResponse, ApiError>,
    selected_metadata: RwSignal<Option<serde_json::Value>>,
) -> View {
    match result {
        Ok(response) => {
            if response.items.is_empty() {
                view! {
                    <tr>
                        <td colspan="6" class="p-4">
                            <EmptyState
                                title="ログがありません"
                                description="検索条件に一致する監査ログは見つかりませんでした。"
                            />
                        </td>
                    </tr>
                }
                .into_view()
            } else {
                response
                    .items
                    .into_iter()
                    .map(|log| render_log_row(log, selected_metadata))
                    .collect_view()
            }
        }
        Err(error) => {
            view! { <tr><td colspan="6" class="p-4 text-center text-status-error-text">{error}</td></tr> }
                .into_view()
        }
    }
}

#[component]
pub fn AdminAuditLogsPage() -> impl IntoView {
    let vm = use_audit_log_view_model();
    let (auth, _) = use_auth();
    let selected_metadata = create_rw_signal::<Option<serde_json::Value>>(None);
    let is_system_admin = create_memo(move |_| {
        auth.get()
            .user
            .as_ref()
            .map(|u| u.is_system_admin)
            .unwrap_or(false)
    });

    let is_filters_default = create_memo(move |_| vm.filters.get() == AuditLogFilters::default());

    let on_filter_change = move |ev: web_sys::Event, field: &str| {
        let mut page = vm.page.get_untracked();
        vm.filters.update(|filters| {
            apply_filter_change(filters, &mut page, field, event_target_value(&ev));
        });
        vm.page.set(page);
    };

    view! {
        <Layout>
            <Show
                when=move || is_system_admin.get()
                fallback=move || view! { <div class="p-6 bg-surface-elevated text-fg shadow rounded">"権限がありません"</div> }
            >
                <div class="space-y-6">
                    <div>
                        <h1 class="text-2xl font-bold text-fg">"監査ログ"</h1>
                        <p class="mt-1 text-sm text-fg-muted">"システム操作の履歴を確認・エクスポートします。"</p>
                    </div>

                    <Show when=move || vm.pii_masked.get()>
                        <div class="rounded-lg border border-status-warning-border bg-status-warning-bg px-3 py-2 text-sm text-status-warning-text">
                            {"表示中の監査ログは個人情報がマスキングされています。"}
                        </div>
                    </Show>

                    <div class="bg-surface-elevated p-4 rounded-lg shadow space-y-4">
                        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-4">
                            <div>
                                <label class="block text-sm font-medium text-fg-muted">"日時 (From)"</label>
                                <input type="datetime-local" class="mt-1 block w-full rounded-md border-form-control-border bg-form-control-bg text-form-control-text shadow-sm sm:text-sm border px-2 py-1"
                                    prop:value=move || vm.filters.get().from
                                    on:input=move |ev| on_filter_change(ev, "from")
                                />
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-fg-muted">"日時 (To)"</label>
                                <input type="datetime-local" class="mt-1 block w-full rounded-md border-form-control-border bg-form-control-bg text-form-control-text shadow-sm sm:text-sm border px-2 py-1"
                                    prop:value=move || vm.filters.get().to
                                    on:input=move |ev| on_filter_change(ev, "to")
                                />
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-fg-muted">"ユーザーID"</label>
                                <input type="text" class="mt-1 block w-full rounded-md border-form-control-border bg-form-control-bg text-form-control-text shadow-sm sm:text-sm border px-2 py-1"
                                    prop:value=move || vm.filters.get().actor_id
                                    on:input=move |ev| on_filter_change(ev, "actor_id")
                                />
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-fg-muted">"イベントタイプ"</label>
                                <select class="mt-1 block w-full rounded-md border-form-control-border bg-form-control-bg text-form-control-text shadow-sm sm:text-sm border px-2 py-1"
                                    prop:value=move || vm.filters.get().event_type
                                    on:change=move |ev| on_filter_change(ev, "event_type")
                                >
                                    <option value="">"すべて"</option>
                                    {AUDIT_EVENT_TYPES.iter().map(|(val, label)| view! {
                                        <option value=*val>{*label}</option>
                                    }).collect_view()}
                                </select>
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-fg-muted">"結果"</label>
                                <select class="mt-1 block w-full rounded-md border-form-control-border bg-form-control-bg text-form-control-text shadow-sm sm:text-sm border px-2 py-1"
                                    prop:value=move || vm.filters.get().result
                                    on:change=move |ev| on_filter_change(ev, "result")
                                >
                                    <option value="">"すべて"</option>
                                    <option value="success">"成功"</option>
                                    <option value="failure">"失敗"</option>
                                </select>
                            </div>
                        </div>
                        <div class="flex justify-end items-center gap-4">
                            <button
                                class="text-sm text-fg-muted hover:text-fg disabled:opacity-50 disabled:cursor-not-allowed"
                                disabled=move || is_filters_default.get()
                                on:click=move |_| {
                                    let mut page = vm.page.get_untracked();
                                    vm.filters.update(|filters| {
                                        clear_filters_and_reset_page(filters, &mut page);
                                    });
                                    vm.page.set(page);
                                }
                            >
                                "フィルタをクリア"
                            </button>
                            <Button
                                variant=ButtonVariant::Primary
                                on:click=move |_| vm.export_action.dispatch(())
                                disabled=Signal::derive(move || vm.export_action.pending().get())
                                loading=Signal::derive(move || vm.export_action.pending().get())
                            >
                                {move || if vm.export_action.pending().get() { "エクスポート中..." } else { "JSONエクスポート" }}
                            </Button>
                        </div>
                    </div>

                    <div class="bg-surface-elevated shadow overflow-hidden sm:rounded-lg overflow-x-auto">
                        <table class="min-w-full divide-y divide-border">
                            <thead class="bg-surface-muted">
                                <tr>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">"日時"</th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">"ユーザー"</th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">"イベント"</th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">"対象"</th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">"結果"</th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">"詳細"</th>
                                </tr>
                            </thead>
                            <tbody class="bg-surface-elevated divide-y divide-border">
                                <Suspense fallback=move || view! { <tr><td colspan="6" class="p-4 text-center">"読み込み中..."</td></tr> }>
                                    {move || vm.logs_resource.get().map(|res| render_logs_content(res, selected_metadata))}
                                </Suspense>
                            </tbody>
                        </table>
                    </div>

                    <div class="mt-4 flex justify-between items-center">
                        <button
                            class="px-4 py-2 border border-border rounded disabled:opacity-50 text-sm text-fg"
                            disabled=move || vm.page.get() <= 1
                            on:click=move |_| vm.page.update(|page| *page = previous_page(*page))
                        >
                            "前へ"
                        </button>
                        <div class="text-sm text-fg">
                            "ページ " {move || vm.page.get()}
                             {move || page_summary_from_logs(vm.logs_resource.get())}
                        </div>
                        <button
                            class="px-4 py-2 border border-border rounded disabled:opacity-50 text-sm text-fg"
                            disabled=move || is_next_page_disabled(vm.page.get(), vm.logs_resource.get())
                            on:click=move |_| vm.page.update(|page| *page = next_page(*page))
                        >
                            "次へ"
                        </button>
                    </div>

                    <Show when=move || selected_metadata.get().is_some()>
                        <div class="fixed inset-0 bg-overlay-backdrop flex items-center justify-center z-50 p-4">
                            <div class="bg-surface-elevated rounded-lg shadow-xl w-full max-w-2xl flex flex-col max-h-[90vh]">
                                <div class="p-4 border-b border-border flex justify-between items-center">
                                    <h3 class="text-lg font-bold text-fg">"メタデータ詳細"</h3>
                                    <button class="text-fg-muted hover:text-fg" on:click=move |_| selected_metadata.update(clear_selected_metadata)>
                                        <i class="fas fa-times"></i>
                                    </button>
                                </div>
                                <div class="p-4 overflow-auto flex-1 bg-surface-muted font-mono text-xs sm:text-sm text-fg">
                                    <pre>{move || metadata_modal_pretty(selected_metadata.get())}</pre>
                                </div>
                                <div class="p-4 border-t border-border flex justify-end">
                                    <button
                                        class="px-4 py-2 bg-surface-muted hover:bg-surface-elevated rounded text-sm font-medium text-fg"
                                        on:click=move |_| selected_metadata.update(clear_selected_metadata)
                                    >
                                        "閉じる"
                                    </button>
                                </div>
                            </div>
                        </div>
                    </Show>
                </div>
            </Show>
        </Layout>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::api::test_support::mock::*;
    use crate::api::{ApiError, AuditLog, AuditLogListResponse};
    use crate::test_support::helpers::{admin_user, provide_auth, regular_user};
    use crate::test_support::ssr::{render_to_string, with_runtime};
    use chrono::{DateTime, Utc};

    fn sample_log_response(
        items: Vec<AuditLog>,
        total: i64,
        per_page: i64,
    ) -> AuditLogListResponse {
        AuditLogListResponse {
            page: 1,
            per_page,
            total,
            items,
        }
    }

    fn sample_audit_log(
        actor_id: Option<&str>,
        result: &str,
        error_code: Option<&str>,
        metadata: Option<serde_json::Value>,
    ) -> AuditLog {
        let occurred_at = DateTime::parse_from_rfc3339("2026-01-17T09:30:15Z")
            .expect("valid datetime")
            .with_timezone(&Utc);
        AuditLog {
            id: "log-1".to_string(),
            occurred_at,
            actor_id: actor_id.map(|value| value.to_string()),
            actor_type: "system_admin".to_string(),
            event_type: "admin_user_create".to_string(),
            target_type: Some("user".to_string()),
            target_id: Some("u-100".to_string()),
            result: result.to_string(),
            error_code: error_code.map(|value| value.to_string()),
            metadata,
            ip: Some("127.0.0.1".to_string()),
            user_agent: Some("Mozilla/5.0".to_string()),
            request_id: Some("req-1".to_string()),
        }
    }

    fn render_logs_html(result: Result<AuditLogListResponse, ApiError>) -> String {
        with_runtime(|| {
            let selected_metadata = create_rw_signal(None::<serde_json::Value>);
            render_logs_content(result, selected_metadata)
                .render_to_string()
                .to_string()
        })
    }

    #[test]
    fn audit_logs_page_renders_denied_for_non_admin() {
        let html = render_to_string(move || {
            provide_auth(Some(regular_user()));
            view! { <AdminAuditLogsPage /> }
        });
        assert!(html.contains("権限がありません"));
    }

    #[test]
    fn audit_logs_page_renders_empty_state_for_admin() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/admin/audit-logs");
            then.status(200).json_body(serde_json::json!({
                "page": 1,
                "per_page": 20,
                "total": 0,
                "items": []
            }));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/admin/audit-logs/export");
            then.status(200).json_body(serde_json::json!([]));
        });

        let server = server.clone();
        let html = render_to_string(move || {
            provide_auth(Some(admin_user(true)));
            provide_context(crate::api::ApiClient::new_with_base_url(
                &server.url("/api"),
            ));
            view! { <AdminAuditLogsPage /> }
        });
        assert!(html.contains("監査ログ"));
        assert!(html.contains("JSONエクスポート"));
    }

    #[test]
    fn helper_filter_and_pagination_logic() {
        let mut filters = AuditLogFilters::default();
        let mut page = 5;
        update_filter_value(&mut filters, "from", "2026-01-01T00:00".into());
        update_filter_value(&mut filters, "to", "2026-01-31T23:59".into());
        update_filter_value(&mut filters, "actor_id", "admin-1".into());
        update_filter_value(&mut filters, "event_type", "admin_user_create".into());
        update_filter_value(&mut filters, "result", "success".into());
        update_filter_value(&mut filters, "unknown", "ignored".into());
        assert_eq!(filters.from, "2026-01-01T00:00");
        assert_eq!(filters.to, "2026-01-31T23:59");
        assert_eq!(filters.actor_id, "admin-1");
        assert_eq!(filters.event_type, "admin_user_create");
        assert_eq!(filters.result, "success");
        assert!(compute_total_pages(0, 20) == 1);
        assert!(compute_total_pages(10, 0) == 1);
        assert!(compute_total_pages(10, -5) == 1);
        assert!(compute_total_pages(41, 20) == 3);
        assert!(can_go_next(1, 41, 20));
        assert!(!can_go_next(3, 41, 20));
        assert!(!can_go_next(1, 0, 20));
        assert!(!can_go_next(1, 10, 0));
        assert_eq!(page_summary_label(41, 20), " / 3");
        assert_eq!(page_summary_label(10, 0), " / 1");

        apply_filter_change(&mut filters, &mut page, "actor_id", "admin-2".into());
        assert_eq!(filters.actor_id, "admin-2");
        assert_eq!(page, 1);

        clear_filters_and_reset_page(&mut filters, &mut page);
        assert_eq!(filters, AuditLogFilters::default());
        assert_eq!(page, 1);

        assert_eq!(previous_page(1), 1);
        assert_eq!(previous_page(4), 3);
        assert_eq!(next_page(4), 5);
    }

    #[test]
    fn helper_metadata_and_badge_logic() {
        let short = serde_json::json!({"a":"b"});
        let long = serde_json::json!({"k":"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"});
        let unicode = serde_json::json!({"message":"こんにちは世界こんにちは世界"});
        assert!(metadata_preview(&short, 50).contains("\"a\""));
        assert!(metadata_preview(&long, 10).ends_with("..."));
        assert!(metadata_preview(&unicode, 8).ends_with("..."));
        assert_eq!(metadata_preview(&short, 0), "...");
        assert!(result_badge_class("success").contains("status-success"));
        assert!(result_badge_class("failure").contains("status-error"));

        let metadata = serde_json::json!({"k":"abcdefghijklmnopqrstuvwxyz"});
        let (preview, raw) =
            metadata_button_payload(Some(metadata.clone()), 10).expect("metadata payload");
        assert!(preview.ends_with("..."));
        assert_eq!(raw, metadata);
        assert!(metadata_button_payload(None, 10).is_none());

        assert_eq!(actor_label(Some("admin-1".to_string())), "admin-1");
        assert_eq!(actor_label(None), "-");
        assert_eq!(
            target_label(Some("user".to_string()), Some("u-1".to_string())),
            "user u-1"
        );
        assert_eq!(target_label(None, None), " ");

        assert_eq!(metadata_modal_pretty(None), "");
        assert!(metadata_modal_pretty(Some(serde_json::json!({"id": 1}))).contains("\"id\""));
        let mut selected = Some(serde_json::json!({"id": "value"}));
        clear_selected_metadata(&mut selected);
        assert!(selected.is_none());
    }

    #[test]
    fn helper_page_resource_logic() {
        let logs = sample_log_response(vec![], 41, 20);
        assert_eq!(page_summary_from_logs(Some(Ok(logs.clone()))), " / 3");
        assert_eq!(
            page_summary_from_logs(Some(Err(ApiError::unknown("failed")))),
            ""
        );
        assert_eq!(page_summary_from_logs(None), "");

        assert!(!is_next_page_disabled(1, Some(Ok(logs.clone()))));
        assert!(is_next_page_disabled(3, Some(Ok(logs))));
        assert!(is_next_page_disabled(
            1,
            Some(Err(ApiError::unknown("failed")))
        ));
        assert!(is_next_page_disabled(1, None));
    }

    #[test]
    fn render_logs_content_covers_empty_rows_and_error_branches() {
        let empty_html = render_logs_html(Ok(sample_log_response(vec![], 0, 20)));
        assert!(empty_html.contains("ログがありません"));

        let row_html = render_logs_html(Ok(sample_log_response(
            vec![sample_audit_log(
                Some("admin-1"),
                "failure",
                Some("AUTH_001"),
                Some(serde_json::json!({"key":"value"})),
            )],
            1,
            20,
        )));
        assert!(row_html.contains("2026-01-17 09:30:15"));
        assert!(row_html.contains("admin-1"));
        assert!(row_html.contains("system_admin"));
        assert!(row_html.contains("admin_user_create"));
        assert!(row_html.contains("user u-100"));
        assert!(row_html.contains("failure"));
        assert!(row_html.contains("AUTH_001"));
        assert!(row_html.contains("key"));

        let fallback_html = render_logs_html(Ok(sample_log_response(
            vec![sample_audit_log(None, "success", None, None)],
            1,
            20,
        )));
        assert!(fallback_html.contains("-"));
        assert!(fallback_html.contains("success"));

        let error_html = render_logs_html(Err(ApiError::unknown("fetch failed")));
        assert!(error_html.contains("fetch failed"));
    }
}
