use crate::api::{AdminAttendanceUpsert, AdminBreakItem, ApiClient};
use crate::components::layout::*;
use chrono::{NaiveDate, NaiveDateTime};
use leptos::*;
use serde_json::json;

fn parse_dt_local(s: &str) -> Option<NaiveDateTime> {
    if s.len() == 16 {
        NaiveDateTime::parse_from_str(&format!("{}:00", s), "%Y-%m-%dT%H:%M:%S").ok()
    } else {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").ok()
    }
}

#[component]
pub fn AdminPage() -> impl IntoView {
    // Requests list/filter
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

    create_effect(move |_| {
        load_list();
    });

    // Request detail modal
    let show_modal = create_rw_signal(false);
    let modal_kind = create_rw_signal(String::new());
    let modal_data = create_rw_signal(serde_json::Value::Null);
    let modal_comment = create_rw_signal(String::new());

    let open_modal = move |kind: String, data: serde_json::Value| {
        modal_kind.set(kind);
        modal_data.set(data);
        modal_comment.set(String::new());
        show_modal.set(true);
    };

    let on_approve = {
        let modal_data = modal_data.clone();
        let modal_comment = modal_comment.clone();
        let show_modal = show_modal.clone();
        move |_| {
            let id = modal_data
                .get()
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let comment = modal_comment.get();
            leptos::spawn_local(async move {
                let api = ApiClient::new();
                let _ = api.admin_approve_request(&id, &comment).await;
            });
            show_modal.set(false);
            load_list();
        }
    };

    let on_reject = {
        let modal_data = modal_data.clone();
        let modal_comment = modal_comment.clone();
        let show_modal = show_modal.clone();
        move |_| {
            let id = modal_data
                .get()
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let comment = modal_comment.get();
            leptos::spawn_local(async move {
                let api = ApiClient::new();
                let _ = api.admin_reject_request(&id, &comment).await;
            });
            show_modal.set(false);
            load_list();
        }
    };

    // Attendance upsert form
    let att_user = create_rw_signal(String::new());
    let att_date = create_rw_signal(String::new());
    let att_in = create_rw_signal(String::new()); // datetime-local
    let att_out = create_rw_signal(String::new()); // datetime-local
    let breaks = create_rw_signal(Vec::<(String, String)>::new());

    let add_break = {
        let breaks = breaks.clone();
        move |_| {
            let mut v = breaks.get();
            v.push((String::new(), String::new()));
            breaks.set(v);
        }
    };

    let on_submit_att = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let user_id = att_user.get();
        let date = att_date.get();
        let cin = parse_dt_local(&att_in.get());
        let cout = if att_out.get().is_empty() {
            None
        } else {
            parse_dt_local(&att_out.get())
        };
        if user_id.is_empty() || date.is_empty() || cin.is_none() {
            return;
        }
        let mut bitems: Vec<AdminBreakItem> = vec![];
        for (s, e) in breaks.get() {
            if s.is_empty() {
                continue;
            }
            let bs = parse_dt_local(&s);
            let be = if e.is_empty() {
                None
            } else {
                parse_dt_local(&e)
            };
            if let Some(bs) = bs {
                bitems.push(AdminBreakItem {
                    break_start_time: bs,
                    break_end_time: be,
                });
            }
        }
        let date_nd = NaiveDate::parse_from_str(&date, "%Y-%m-%d").ok();
        if let (Some(date), Some(cin)) = (date_nd, cin) {
            let payload = AdminAttendanceUpsert {
                user_id,
                date,
                clock_in_time: cin,
                clock_out_time: cout,
                breaks: if bitems.is_empty() {
                    None
                } else {
                    Some(bitems)
                },
            };
            leptos::spawn_local(async move {
                let api = ApiClient::new();
                let _ = api.admin_upsert_attendance(payload).await;
            });
        }
    };

    // Force end break quick tool
    let feb_id = create_rw_signal(String::new());
    let on_force_end = move |_| {
        let id = feb_id.get();
        if id.is_empty() {
            return;
        }
        leptos::spawn_local(async move {
            let api = ApiClient::new();
            let _ = api.admin_force_end_break(&id).await;
        });
    };

    view! {
        <Layout>
            <div class="space-y-6">
                <div>
                    <h1 class="text-2xl font-bold text-gray-900">{"管理者画面"}</h1>
                    <p class="mt-1 text-sm text-gray-600">{"申請の承認/却下、勤怠の手動登録ができます。"}</p>
                </div>

                <div class="grid grid-cols-1 gap-6 lg:grid-cols-3">
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
                                                    let _id = data.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                    let statusv = data.get("status").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                    let user = data.get("user_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                    let target = if kind=="leave" { format!("{} - {}", data.get("start_date").and_then(|v| v.as_str()).unwrap_or(""), data.get("end_date").and_then(|v| v.as_str()).unwrap_or("")) } else { data.get("date").and_then(|v| v.as_str()).unwrap_or("").to_string() };
                                                    let open = {
                                                        let kind = kind.clone();
                                                        let data = data.clone();
                                                        move |_| open_modal(kind.clone(), data.clone())
                                                    };
                                                    view!{
                                                        <tr>
                                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{ if kind=="leave" { "休暇" } else { "残業" } }</td>
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

                    <div class="bg-white shadow rounded-lg p-6">
                        <h3 class="text-lg font-medium text-gray-900 mb-4">{"勤怠の手動登録（アップサート）"}</h3>
                        <form class="space-y-3" on:submit=on_submit_att>
                            <input placeholder="User ID" class="w-full border rounded px-2 py-1" on:input=move |ev| att_user.set(event_target_value(&ev)) />
                            <input type="date" class="w-full border rounded px-2 py-1" on:input=move |ev| att_date.set(event_target_value(&ev)) />
                            <input type="datetime-local" class="w-full border rounded px-2 py-1" on:input=move |ev| att_in.set(event_target_value(&ev)) />
                            <input type="datetime-local" class="w-full border rounded px-2 py-1" on:input=move |ev| att_out.set(event_target_value(&ev)) />
                            <div>
                                <div class="flex items-center justify-between mb-1"><span class="text-sm text-gray-700">{"休憩（任意）"}</span><button type="button" class="text-blue-600 text-sm" on:click=add_break>{"行を追加"}</button></div>
                                <For each=move || breaks.get() key=|pair| pair.clone() children=move |(s0,e0)| {
                                    let s = create_rw_signal(s0);
                                    let e = create_rw_signal(e0);
                                    view!{ <div class="flex space-x-2 mb-2"><input type="datetime-local" class="border rounded px-2 py-1 w-full" prop:value=s on:input=move |ev| s.set(event_target_value(&ev)) /><input type="datetime-local" class="border rounded px-2 py-1 w-full" prop:value=e on:input=move |ev| e.set(event_target_value(&ev)) /></div> }
                                } />
                            </div>
                            <button type="submit" class="w-full bg-green-600 text-white rounded py-2">{"登録"}</button>
                        </form>
                        <div class="mt-4">
                            <h4 class="text-sm font-medium text-gray-900 mb-2">{"休憩の強制終了"}</h4>
                            <div class="flex space-x-2">
                                <input placeholder="Break ID" class="border rounded px-2 py-1 w-full" on:input=move |ev| feb_id.set(event_target_value(&ev)) />
                                <button class="px-3 py-1 bg-amber-600 text-white rounded" on:click=on_force_end>{"強制終了"}</button>
                            </div>
                        </div>
                    </div>
                </div>

                <Show when=move || show_modal.get()>
                    <div class="fixed inset-0 bg-black/30 flex items-center justify-center z-50">
                        <div class="bg-white rounded-lg shadow-lg w-full max-w-lg p-6">
                            <h3 class="text-lg font-medium text-gray-900 mb-2">{"申請詳細"}</h3>
                            <pre class="text-xs bg-gray-50 p-2 rounded overflow-auto max-h-64">{format!("{}", modal_data.get())}</pre>
                            <div class="mt-3">
                                <label class="block text-sm font-medium text-gray-700">{"コメント（必須）"}</label>
                                <textarea class="w-full border rounded px-2 py-1" on:input=move |ev| modal_comment.set(event_target_value(&ev))></textarea>
                            </div>
                            <div class="mt-4 flex justify-end space-x-2">
                                <button class="px-3 py-1 rounded border" on:click=move |_| show_modal.set(false)>{"閉じる"}</button>
                                <button class="px-3 py-1 rounded bg-red-600 text-white" on:click=on_reject>{"却下"}</button>
                                <button class="px-3 py-1 rounded bg-green-600 text-white" on:click=on_approve>{"承認"}</button>
                            </div>
                        </div>
                    </div>
                </Show>
            </div>
        </Layout>
    }
}
