use super::view_model::use_audit_log_view_model;
use crate::components::common::{Button, ButtonVariant};
use crate::components::layout::Layout;
use crate::state::auth::use_auth;
use leptos::*;

#[component]
pub fn AdminAuditLogsPage() -> impl IntoView {
    let vm = use_audit_log_view_model();
    let (auth, _) = use_auth();
    let is_system_admin = create_memo(move |_| {
        auth.get()
            .user
            .as_ref()
            .map(|u| u.is_system_admin)
            .unwrap_or(false)
    });

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

                    <div class="bg-white p-4 rounded-lg shadow space-y-4">
                        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-4">
                            <div>
                                <label class="block text-sm font-medium text-gray-700">"日時 (From)"</label>
                                <input type="datetime-local" class="mt-1 block w-full rounded-md border-gray-300 shadow-sm sm:text-sm border px-2 py-1"
                                    prop:value=move || vm.filters.get().from
                                    on:input=move |ev| on_filter_change(ev, "from")
                                />
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-gray-700">"日時 (To)"</label>
                                <input type="datetime-local" class="mt-1 block w-full rounded-md border-gray-300 shadow-sm sm:text-sm border px-2 py-1"
                                    prop:value=move || vm.filters.get().to
                                    on:input=move |ev| on_filter_change(ev, "to")
                                />
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-gray-700">"ユーザーID"</label>
                                <input type="text" class="mt-1 block w-full rounded-md border-gray-300 shadow-sm sm:text-sm border px-2 py-1"
                                    prop:value=move || vm.filters.get().actor_id
                                    on:input=move |ev| on_filter_change(ev, "actor_id")
                                />
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-gray-700">"イベントタイプ"</label>
                                <input type="text" class="mt-1 block w-full rounded-md border-gray-300 shadow-sm sm:text-sm border px-2 py-1"
                                    prop:value=move || vm.filters.get().event_type
                                    on:input=move |ev| on_filter_change(ev, "event_type")
                                />
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-gray-700">"結果"</label>
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
                        <div class="flex justify-end">
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

                    <div class="bg-white shadow overflow-hidden sm:rounded-lg overflow-x-auto">
                        <table class="min-w-full divide-y divide-gray-200">
                            <thead class="bg-gray-50">
                                <tr>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"日時"</th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"ユーザー"</th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"イベント"</th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"対象"</th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"結果"</th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"詳細"</th>
                                </tr>
                            </thead>
                            <tbody class="bg-white divide-y divide-gray-200">
                                <Suspense fallback=move || view! { <tr><td colspan="6" class="p-4 text-center">"読み込み中..."</td></tr> }>
                                    {move || vm.logs_resource.get().map(|res| match res {
                                        Ok(response) => {
                                            if response.items.is_empty() {
                                                view! { <tr><td colspan="6" class="p-4 text-center">"ログがありません"</td></tr> }.into_view()
                                            } else {
                                                response.items.into_iter().map(|log| {
                                                    view! {
                                                        <tr>
                                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                                                                {log.occurred_at.format("%Y-%m-%d %H:%M:%S").to_string()}
                                                            </td>
                                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                                                                {log.actor_id.unwrap_or_else(|| "-".to_string())}
                                                                <span class="text-xs text-gray-500 block">{log.actor_type}</span>
                                                            </td>
                                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                                                                {log.event_type}
                                                            </td>
                                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                                                                {format!("{} {}", log.target_type.unwrap_or_default(), log.target_id.unwrap_or_default())}
                                                            </td>
                                                             <td class="px-6 py-4 whitespace-nowrap text-sm">
                                                                <span class={if log.result == "success" { "px-2 inline-flex text-xs leading-5 font-semibold rounded-full bg-green-100 text-green-800" } else { "px-2 inline-flex text-xs leading-5 font-semibold rounded-full bg-red-100 text-red-800" }}>
                                                                    {log.result.clone()}
                                                                </span>
                                                                {log.error_code.clone().map(|c| view! { <span class="block text-xs text-red-500">{c}</span> })}
                                                            </td>
                                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                                                                // Simple metadata display
                                                                {log.metadata.map(|m| {
                                                                    let s = m.to_string();
                                                                    if s.len() > 50 {
                                                                        format!("{}...", &s[0..50])
                                                                    } else {
                                                                        s
                                                                    }
                                                                }).unwrap_or_default()}
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
                </div>
            </Show>
        </Layout>
    }
}
