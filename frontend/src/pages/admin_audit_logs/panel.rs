use super::view_model::{use_audit_log_view_model, AuditLogFilters, AUDIT_EVENT_TYPES};
use crate::components::common::{Button, ButtonVariant};
use crate::components::empty_state::EmptyState;
use crate::components::layout::Layout;
use crate::state::auth::use_auth;
use leptos::*;

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
        vm.filters.update(|f| match field {
            "from" => f.from = val,
            "to" => f.to = val,
            "actor_id" => f.actor_id = val,
            "event_type" => f.event_type = val,
            "result" => f.result = val,
            _ => {}
        });
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
                                                                <span class={if log.result == "success" { "px-2 inline-flex text-xs leading-5 font-semibold rounded-full bg-status-success-bg text-status-success-text" } else { "px-2 inline-flex text-xs leading-5 font-semibold rounded-full bg-status-error-bg text-status-error-text" }}>
                                                                    {log.result.clone()}
                                                                </span>
                                                                {log.error_code.clone().map(|c| view! { <span class="block text-xs text-status-error-text">{c}</span> })}
                                                            </td>
                                                             <td class="px-6 py-4 whitespace-nowrap text-sm text-fg-muted">
                                                                 {
                                                                    let m = log.metadata.clone();
                                                                    m.map(|val| {
                                                                        let s = val.to_string();
                                                                        let display = if s.len() > 50 {
                                                                            format!("{}...", &s[0..50])
                                                                        } else {
                                                                            s
                                                                        };
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
                             {move || vm.logs_resource.get().map(|res| if let Ok(r) = res { format!(" / {}", (r.total as f64 / r.per_page as f64).ceil() as i64) } else { "".to_string() })}
                        </div>
                        <button
                            class="px-4 py-2 border border-border rounded disabled:opacity-50 text-sm text-fg"
                            disabled=move || {
                                vm.logs_resource.get().map(|res| if let Ok(r) = res {
                                    vm.page.get() >= (r.total as f64 / r.per_page as f64).ceil() as i64
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
            provide_context(crate::api::ApiClient::new_with_base_url(&server.url("/api")));
            view! { <AdminAuditLogsPage /> }
        });
        assert!(html.contains("監査ログ"));
        assert!(html.contains("JSONエクスポート"));
    }
}
