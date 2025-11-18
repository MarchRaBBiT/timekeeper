use crate::api::ApiClient;
use leptos::*;
use serde_json::json;

#[component]
pub fn AdminRequestsSection(admin_allowed: Memo<bool>) -> impl IntoView {
    let status = create_rw_signal(String::new());
    let user_id = create_rw_signal(String::new());
    let page = create_rw_signal(1u32);
    let per_page = create_rw_signal(20u32);
    let list = create_rw_signal(serde_json::Value::Null);

    let load_list = {
        let status = status.clone();
        let user_id = user_id.clone();
        let page = page.clone();
        let per_page = per_page.clone();
        let list = list.clone();
        move || {
            leptos::spawn_local(async move {
                let api = ApiClient::new();
                let s_owned = status.get();
                let u_owned = user_id.get();
                let s = if s_owned.is_empty() {
                    None
                } else {
                    Some(s_owned.as_str())
                };
                let u = if u_owned.is_empty() {
                    None
                } else {
                    Some(u_owned.as_str())
                };
                if let Ok(v) = api
                    .admin_list_requests(s, u, Some(page.get()), Some(per_page.get()))
                    .await
                {
                    list.set(v);
                }
            });
        }
    };

    {
        let load_list_cb = load_list.clone();
        let admin_allowed = admin_allowed.clone();
        create_effect(move |_| {
            if !admin_allowed.get() {
                return;
            }
            load_list_cb();
        });
    }

    let show_modal = create_rw_signal(false);
    let modal_data = create_rw_signal(serde_json::Value::Null);
    let modal_comment = create_rw_signal(String::new());

    let open_modal = {
        let show_modal = show_modal.clone();
        let modal_data = modal_data.clone();
        let modal_comment = modal_comment.clone();
        move |kind: String, data: serde_json::Value| {
            let _ = kind;
            modal_data.set(data);
            modal_comment.set(String::new());
            show_modal.set(true);
        }
    };

    let on_action = {
        let modal_data = modal_data.clone();
        let modal_comment = modal_comment.clone();
        let show_modal = show_modal.clone();
        let load_list = load_list.clone();
        move |approve: bool| {
            let modal_data = modal_data.get();
            let id = modal_data
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let comment = modal_comment.get();
            leptos::spawn_local(async move {
                let api = ApiClient::new();
                if approve {
                    let _ = api.admin_approve_request(&id, &comment).await;
                } else {
                    let _ = api.admin_reject_request(&id, &comment).await;
                }
            });
            show_modal.set(false);
            load_list();
        }
    };

    view! {
        <>
            <div class="bg-white shadow rounded-lg p-6 lg:col-span-2">
                <h3 class="text-lg font-medium text-gray-900 mb-4">{"申請一覧"}</h3>
                <div class="flex space-x-3 mb-4">
                    <select class="border-gray-300 rounded-md" on:change=move |ev| { status.set(event_target_value(&ev)); load_list(); }>
                        <option value="">{"すべて"}</option>
                        <option value="pending">{"承認待ち"}</option>
                        <option value="approved">{"承認済"}</option>
                        <option value="rejected">{"却下"}</option>
                        <option value="cancelled">{"取消"}</option>
                    </select>
                    <input placeholder="User ID" class="border rounded-md px-2" on:input=move |ev| user_id.set(event_target_value(&ev)) />
                    <button class="px-3 py-1 bg-blue-600 text-white rounded" on:click=move |_| load_list()>{"検索"}</button>
                </div>
                <div class="overflow-x-auto">
                    <table class="min-w-full divide-y divide-gray-200">
                        <thead class="bg-gray-50">
                            <tr>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"種類"}</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"対象"}</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"申請者"}</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"状態"}</th>
                                <th class="px-6 py-3"/>
                            </tr>
                        </thead>
                        <tbody class="bg-white divide-y divide-gray-200">
                            <Show when=move || list.get().is_object()>
                                {let data = list.get();
                                    let leaves = data.get("leave_requests").cloned().unwrap_or(json!([]));
                                    let ots = data.get("overtime_requests").cloned().unwrap_or(json!([]));
                                    let mut rows: Vec<serde_json::Value> = vec![];
                                    if let Some(arr) = leaves.as_array() { for r in arr { rows.push(json!({"kind":"leave","data": r})); } }
                                    if let Some(arr) = ots.as_array() { for r in arr { rows.push(json!({"kind":"overtime","data": r})); } }
                                    view!{ <>{
                                        rows.into_iter().map(|row| {
                                            let kind = row.get("kind").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                            let data = row.get("data").cloned().unwrap_or(json!({}));
                                            let statusv = data.get("status").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                            let user = data.get("user_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                            let target = if kind=="leave" { format!("{} - {}", data.get("start_date").and_then(|v| v.as_str()).unwrap_or(""), data.get("end_date").and_then(|v| v.as_str()).unwrap_or("")) } else { data.get("date").and_then(|v| v.as_str()).unwrap_or("").to_string() };
                                            let kind_for_modal = kind.clone();
                                            let kind_label = if kind == "leave" { "休暇" } else { "残業" };
                                            let open = {
                                                let data = data.clone();
                                                move |_| open_modal(kind_for_modal.clone(), data.clone())
                                            };
                                            view!{
                                                <tr>
                                                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{kind_label}</td>
                                                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{target}</td>
                                                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{user}</td>
                                                    <td class="px-6 py-4 whitespace-nowrap"><span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-gray-100 text-gray-800">{statusv.clone()}</span></td>
                                                    <td class="px-6 py-4 whitespace-nowrap text-right text-sm">
                                                        <button class="text-blue-600" on:click=open>{"詳細"}</button>
                                                    </td>
                                                </tr>
                                            }
                                        }).collect::<Vec<_>>()
                                    }</> }
                                }
                            </Show>
                        </tbody>
                    </table>
                </div>
            </div>
            <Show when=move || show_modal.get()>
                <div class="fixed inset-0 bg-black/30 flex items-center justify-center z-50">
                    <div class="bg-white rounded-lg shadow-lg w-full max-w-lg p-6">
                        <h3 class="text-lg font-medium text-gray-900 mb-2">{"申請詳細"}</h3>
                        <pre class="text-xs bg-gray-50 p-2 rounded overflow-auto max-h-64">{format!("{}", modal_data.get())}</pre>
                        <div class="mt-3">
                            <label class="block text-sm font-medium text-gray-700">{"コメント（任意）"}</label>
                            <textarea class="w-full border rounded px-2 py-1" on:input=move |ev| modal_comment.set(event_target_value(&ev))></textarea>
                        </div>
                        <div class="mt-4 flex justify-end space-x-2">
                            <button class="px-3 py-1 rounded border" on:click=move |_| show_modal.set(false)>{"閉じる"}</button>
                            <button class="px-3 py-1 rounded bg-red-600 text-white" on:click=move |_| on_action(false)>{"却下"}</button>
                            <button class="px-3 py-1 rounded bg-green-600 text-white" on:click=move |_| on_action(true)>{"承認"}</button>
                        </div>
                    </div>
                </div>
            </Show>
        </>
    }
}
