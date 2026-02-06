use super::view_model::{use_audit_log_view_model, AuditLogFilters, AUDIT_EVENT_TYPES};
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
        let val = event_target_value(&ev);
        vm.filters.update(|f| update_filter_value(f, field, val));
        vm.page.set(1);
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
                                    vm.filters.set(AuditLogFilters::default());
                                    vm.page.set(1);
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
                                    {move || vm.logs_resource.get().map(|res| match res {
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
                                                }.into_view()
                                            } else {
                                                response.items.into_iter().map(|log| {
                                                    view! {
                                                        <tr>
                                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-fg-muted">
                                                                {log.occurred_at.format("%Y-%m-%d %H:%M:%S").to_string()}
                                                            </td>
                                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-fg">
                                                                {log.actor_id.unwrap_or_else(|| "-".to_string())}
                                                                <span class="text-xs text-fg-muted block">{log.actor_type}</span>
                                                            </td>
                                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-fg">
                                                                {log.event_type}
                                                            </td>
                                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-fg-muted">
                                                                {format!("{} {}", log.target_type.unwrap_or_default(), log.target_id.unwrap_or_default())}
                                                            </td>
                                                             <td class="px-6 py-4 whitespace-nowrap text-sm">
                                                                <span class={result_badge_class(&log.result)}>
                                                                    {log.result.clone()}
                                                                </span>
                                                                {log.error_code.clone().map(|c| view! { <span class="block text-xs text-status-error-text">{c}</span> })}
                                                            </td>
                                                             <td class="px-6 py-4 whitespace-nowrap text-sm text-fg-muted">
                                                                 {
                                                                    let m = log.metadata.clone();
                                                                    m.map(|val| {
                                                                        let display = metadata_preview(&val, 50);
                                                                        let val_clone = val.clone();
                                                                        view! {
                                                                            <button
                                                                                class="text-link hover:text-link-hover hover:underline text-left font-mono text-xs"
                                                                                on:click=move |_| selected_metadata.set(Some(val_clone.clone()))
                                                                            >
                                                                                {display}
                                                                            </button>
                                                                        }
                                                                    })
                                                                }
                                                            </td>
                                                        </tr>
                                                    }
                                                }).collect_view()
                                            }
                                        }
                                        Err(e) => view! { <tr><td colspan="6" class="p-4 text-center text-status-error-text">{e}</td></tr> }.into_view()
                                    })}
                                </Suspense>
                            </tbody>
                        </table>
                    </div>

                    <div class="mt-4 flex justify-between items-center">
                        <button
                            class="px-4 py-2 border border-border rounded disabled:opacity-50 text-sm text-fg"
                            disabled=move || vm.page.get() <= 1
                            on:click=move |_| vm.page.update(|p| *p = (*p - 1).max(1))
                        >
                            "前へ"
                        </button>
                        <div class="text-sm text-fg">
                            "ページ " {move || vm.page.get()}
                             {move || vm.logs_resource.get().map(|res| if let Ok(r) = res { page_summary_label(r.total, r.per_page) } else { "".to_string() })}
                        </div>
                        <button
                            class="px-4 py-2 border border-border rounded disabled:opacity-50 text-sm text-fg"
                            disabled=move || {
                                vm.logs_resource.get().map(|res| if let Ok(r) = res {
                                    !can_go_next(vm.page.get(), r.total, r.per_page)
                                } else {
                                    true
                                }).unwrap_or(true)
                            }
                            on:click=move |_| vm.page.update(|p| *p += 1)
                        >
                            "次へ"
                        </button>
                    </div>

                    <Show when=move || selected_metadata.get().is_some()>
                        <div class="fixed inset-0 bg-overlay-backdrop flex items-center justify-center z-50 p-4">
                            <div class="bg-surface-elevated rounded-lg shadow-xl w-full max-w-2xl flex flex-col max-h-[90vh]">
                                <div class="p-4 border-b border-border flex justify-between items-center">
                                    <h3 class="text-lg font-bold text-fg">"メタデータ詳細"</h3>
                                    <button class="text-fg-muted hover:text-fg" on:click=move |_| selected_metadata.set(None)>
                                        <i class="fas fa-times"></i>
                                    </button>
                                </div>
                                <div class="p-4 overflow-auto flex-1 bg-surface-muted font-mono text-xs sm:text-sm text-fg">
                                    <pre>{move || selected_metadata.get().map(|m| serde_json::to_string_pretty(&m).unwrap_or_default()).unwrap_or_default()}</pre>
                                </div>
                                <div class="p-4 border-t border-border flex justify-end">
                                    <button
                                        class="px-4 py-2 bg-surface-muted hover:bg-surface-elevated rounded text-sm font-medium text-fg"
                                        on:click=move |_| selected_metadata.set(None)
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
    use crate::test_support::helpers::{admin_user, provide_auth, regular_user};
    use crate::test_support::ssr::render_to_string;

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
    }
}
