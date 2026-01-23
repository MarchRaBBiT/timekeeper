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
                fallback=move || view! { <div class="p-6 bg-white shadow rounded">"権限がありません"</div> }
            >
                <div class="space-y-6">
                    <div>
                        <h1 class="text-2xl font-bold text-gray-900">"監査ログ"</h1>
                        <p class="mt-1 text-sm text-gray-600">"システム操作の履歴を確認・エクスポートします。"</p>
                    </div>

                    <div class="bg-white dark:bg-gray-800 p-4 rounded-lg shadow space-y-4">
                        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-4">
                            <div>
                                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">"日時 (From)"</label>
                                <input type="datetime-local" class="mt-1 block w-full rounded-md border-gray-300 shadow-sm sm:text-sm border px-2 py-1"
                                    prop:value=move || vm.filters.get().from
                                    on:input=move |ev| on_filter_change(ev, "from")
                                />
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">"日時 (To)"</label>
                                <input type="datetime-local" class="mt-1 block w-full rounded-md border-gray-300 shadow-sm sm:text-sm border px-2 py-1"
                                    prop:value=move || vm.filters.get().to
                                    on:input=move |ev| on_filter_change(ev, "to")
                                />
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">"ユーザーID"</label>
                                <input type="text" class="mt-1 block w-full rounded-md border-gray-300 shadow-sm sm:text-sm border px-2 py-1"
                                    prop:value=move || vm.filters.get().actor_id
                                    on:input=move |ev| on_filter_change(ev, "actor_id")
                                />
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">"イベントタイプ"</label>
                                <select class="mt-1 block w-full rounded-md border-gray-300 shadow-sm sm:text-sm border px-2 py-1"
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
                                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">"結果"</label>
                                <select class="mt-1 block w-full rounded-md border-gray-300 shadow-sm sm:text-sm border px-2 py-1"
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
                                class="text-sm text-gray-500 hover:text-gray-700 disabled:opacity-50 disabled:cursor-not-allowed"
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

                    <div class="bg-white dark:bg-gray-800 shadow overflow-hidden sm:rounded-lg overflow-x-auto">
                        <table class="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
                            <thead class="bg-gray-50 dark:bg-gray-700">
                                <tr>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-200 uppercase tracking-wider">"日時"</th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-200 uppercase tracking-wider">"ユーザー"</th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-200 uppercase tracking-wider">"イベント"</th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-200 uppercase tracking-wider">"対象"</th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-200 uppercase tracking-wider">"結果"</th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-200 uppercase tracking-wider">"詳細"</th>
                                </tr>
                            </thead>
                            <tbody class="bg-white dark:bg-gray-800 divide-y divide-gray-200 dark:divide-gray-700">
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
                                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                                                {log.occurred_at.format("%Y-%m-%d %H:%M:%S").to_string()}
                                                            </td>
                                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900 dark:text-gray-100">
                                                                {log.actor_id.unwrap_or_else(|| "-".to_string())}
                                                                <span class="text-xs text-gray-500 dark:text-gray-400 block">{log.actor_type}</span>
                                                            </td>
                                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900 dark:text-gray-100">
                                                                {log.event_type}
                                                            </td>
                                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                                                {format!("{} {}", log.target_type.unwrap_or_default(), log.target_id.unwrap_or_default())}
                                                            </td>
                                                             <td class="px-6 py-4 whitespace-nowrap text-sm">
                                                                <span class={if log.result == "success" { "px-2 inline-flex text-xs leading-5 font-semibold rounded-full bg-green-100 text-green-800" } else { "px-2 inline-flex text-xs leading-5 font-semibold rounded-full bg-red-100 text-red-800" }}>
                                                                    {log.result.clone()}
                                                                </span>
                                                                {log.error_code.clone().map(|c| view! { <span class="block text-xs text-red-500">{c}</span> })}
                                                            </td>
                                                             <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
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
                                                                                class="text-blue-600 hover:underline text-left font-mono text-xs"
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
                                        Err(e) => view! { <tr><td colspan="6" class="p-4 text-center text-red-500">{e}</td></tr> }.into_view()
                                    })}
                                </Suspense>
                            </tbody>
                        </table>
                    </div>

                    <div class="mt-4 flex justify-between items-center">
                        <button
                            class="px-4 py-2 border rounded disabled:opacity-50 text-sm"
                            disabled=move || vm.page.get() <= 1
                            on:click=move |_| vm.page.update(|p| *p = (*p - 1).max(1))
                        >
                            "前へ"
                        </button>
                        <div class="text-sm text-gray-700">
                            "ページ " {move || vm.page.get()}
                             {move || vm.logs_resource.get().map(|res| if let Ok(r) = res { format!(" / {}", (r.total as f64 / r.per_page as f64).ceil() as i64) } else { "".to_string() })}
                        </div>
                        <button
                            class="px-4 py-2 border rounded disabled:opacity-50 text-sm"
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
                        <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
                            <div class="bg-white dark:bg-gray-800 rounded-lg shadow-xl w-full max-w-2xl flex flex-col max-h-[90vh]">
                                <div class="p-4 border-b dark:border-gray-700 flex justify-between items-center">
                                    <h3 class="text-lg font-bold dark:text-gray-100">"メタデータ詳細"</h3>
                                    <button class="text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200" on:click=move |_| selected_metadata.set(None)>
                                        <i class="fas fa-times"></i>
                                    </button>
                                </div>
                                <div class="p-4 overflow-auto flex-1 bg-gray-50 dark:bg-gray-900 font-mono text-xs sm:text-sm text-gray-900 dark:text-gray-100">
                                    <pre>{move || selected_metadata.get().map(|m| serde_json::to_string_pretty(&m).unwrap_or_default()).unwrap_or_default()}</pre>
                                </div>
                                <div class="p-4 border-t dark:border-gray-700 flex justify-end">
                                    <button
                                        class="px-4 py-2 bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600 rounded text-sm font-medium dark:text-gray-100"
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
