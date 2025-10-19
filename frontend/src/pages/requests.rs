use crate::api::{ApiClient, CreateLeaveRequest, CreateOvertimeRequest};
use crate::components::layout::*;
use leptos::*;
use serde_json::json;

#[component]
pub fn RequestsPage() -> impl IntoView {
    // signals for simple forms and list refresh
    let leave_type = create_rw_signal(String::from("annual"));
    let leave_start = create_rw_signal(String::new());
    let leave_end = create_rw_signal(String::new());
    let leave_reason = create_rw_signal(String::new());

    let ot_date = create_rw_signal(String::new());
    let ot_hours = create_rw_signal(String::new());
    let ot_reason = create_rw_signal(String::new());

    // simplistic my requests list
    let items = create_rw_signal(serde_json::Value::Null);

    let load_my = {
        let items = items.clone();
        move || {
            leptos::spawn_local(async move {
                let api = ApiClient::new();
                if let Ok(v) = api.get_my_requests().await {
                    items.set(v);
                }
            });
        }
    };

    create_effect(move |_| {
        load_my();
    });

    view! {
        <Layout>
            <div class="space-y-6">
                <div>
                    <h1 class="text-2xl font-bold text-gray-900">{"申請管理"}</h1>
                    <p class="mt-1 text-sm text-gray-600">{"休暇・残業の申請を作成・管理できます。"}</p>
                </div>

                <div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
                    <div class="bg-white shadow rounded-lg p-6">
                        <h3 class="text-lg font-medium text-gray-900 mb-4">{"休暇申請"}</h3>
                        <form class="space-y-4" on:submit=move |ev| {
                            ev.prevent_default();
                            let api = ApiClient::new();
                            let lt = leave_type.get();
                            let sd = chrono::NaiveDate::parse_from_str(&leave_start.get(), "%Y-%m-%d");
                            let ed = chrono::NaiveDate::parse_from_str(&leave_end.get(), "%Y-%m-%d");
                            if let (Ok(start_date), Ok(end_date)) = (sd, ed) {
                                let req = CreateLeaveRequest { leave_type: lt, start_date, end_date, reason: if leave_reason.get().is_empty() { None } else { Some(leave_reason.get()) } };
                                leptos::spawn_local(async move { let _ = api.create_leave_request(req).await; });
                            }
                        }>
                            <div>
                                <label class="block text-sm font-medium text-gray-700">{"種類"}</label>
                                <select class="mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500" on:change=move |ev| leave_type.set(event_target_value(&ev))>
                                    <option value="annual">{"年次有給"}</option>
                                    <option value="sick">{"病気"}</option>
                                    <option value="personal">{"私用"}</option>
                                    <option value="other">{"その他"}</option>
                                </select>
                            </div>
                            <div class="grid grid-cols-2 gap-4">
                                <div>
                                    <label class="block text-sm font-medium text-gray-700">{"開始日"}</label>
                                    <input type="date" class="mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500" on:input=move |ev| leave_start.set(event_target_value(&ev)) />
                                </div>
                                <div>
                                    <label class="block text-sm font-medium text-gray-700">{"終了日"}</label>
                                    <input type="date" class="mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500" on:input=move |ev| leave_end.set(event_target_value(&ev)) />
                                </div>
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-gray-700">{"理由"}</label>
                                <textarea rows="3" class="mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500" placeholder="申請理由を入力してください" on:input=move |ev| leave_reason.set(event_target_value(&ev))></textarea>
                            </div>
                            <button type="submit" class="w-full bg-blue-600 text-white py-2 px-4 rounded-md hover:bg-blue-700">
                                {"申請する"}
                            </button>
                        </form>
                    </div>

                    <div class="bg-white shadow rounded-lg p-6">
                        <h3 class="text-lg font-medium text-gray-900 mb-4">{"残業申請"}</h3>
                        <form class="space-y-4" on:submit=move |ev| {
                            ev.prevent_default();
                            let api = ApiClient::new();
                            if let Ok(date) = chrono::NaiveDate::parse_from_str(&ot_date.get(), "%Y-%m-%d") {
                                let hours = ot_hours.get().parse::<f64>().unwrap_or(0.0);
                                let req = CreateOvertimeRequest { date, planned_hours: hours, reason: if ot_reason.get().is_empty() { None } else { Some(ot_reason.get()) } };
                                leptos::spawn_local(async move { let _ = api.create_overtime_request(req).await; });
                            }
                        }>
                            <div>
                                <label class="block text-sm font-medium text-gray-700">{"日付"}</label>
                                <input type="date" class="mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500" on:input=move |ev| ot_date.set(event_target_value(&ev)) />
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-gray-700">{"申請時間"}</label>
                                <input type="number" step="0.5" class="mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500" placeholder="2.5" on:input=move |ev| ot_hours.set(event_target_value(&ev)) />
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-gray-700">{"理由"}</label>
                                <textarea rows="3" class="mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500" placeholder="申請理由を入力してください" on:input=move |ev| ot_reason.set(event_target_value(&ev))></textarea>
                            </div>
                            <button type="submit" class="w-full bg-blue-600 text-white py-2 px-4 rounded-md hover:bg-blue-700">
                                {"申請する"}
                            </button>
                        </form>
                    </div>
                </div>

                <div class="bg-white shadow rounded-lg p-6">
                    <h3 class="text-lg font-medium text-gray-900 mb-4">{"申請一覧"}</h3>
                    <div class="overflow-x-auto">
                        <table class="min-w-full divide-y divide-gray-200">
                            <thead class="bg-gray-50">
                                <tr>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"種類"}</th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"期間/日付"}</th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"理由"}</th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"状態"}</th>
                                    <th class="px-6 py-3"/>
                                </tr>
                            </thead>
                            <tbody class="bg-white divide-y divide-gray-200">
                                <Show when=move || items.get().is_object()>
                                    {let data = items.get();
                                        let leaves = data.get("leave_requests").cloned().unwrap_or(json!([]));
                                        let ots = data.get("overtime_requests").cloned().unwrap_or(json!([]));
                                        let mut rows: Vec<serde_json::Value> = vec![];
                                        if let Some(arr) = leaves.as_array() { for r in arr { rows.push(json!({"kind":"leave","data": r})); } }
                                        if let Some(arr) = ots.as_array() { for r in arr { rows.push(json!({"kind":"overtime","data": r})); } }
                                        view!{ <>{
                                            rows.into_iter().map(|row| {
                                                let kind = row.get("kind").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                let is_leave = kind == "leave";
                                                let data = row.get("data").cloned().unwrap_or(json!({}));
                                                let id = data.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                let id_sv = store_value(id);
                                                let status = data.get("status").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                let reason = data.get("reason").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                let period = if kind=="leave" { format!("{} - {}", data.get("start_date").and_then(|v| v.as_str()).unwrap_or(""), data.get("end_date").and_then(|v| v.as_str()).unwrap_or("")) } else { data.get("date").and_then(|v| v.as_str()).unwrap_or("").to_string() };
                                                view!{
                                                    <tr>
                                                        <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{ if is_leave { "休暇" } else { "残業" } }</td>
                                                        <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{period}</td>
                                                        <td class="px-6 py-4 text-sm text-gray-900">{reason}</td>
                                                        <td class="px-6 py-4 whitespace-nowrap">
                                                            <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-gray-100 text-gray-800">{status.clone()}</span>
                                                        </td>
                                                        <td class="px-6 py-4 whitespace-nowrap text-right text-sm">
                                                            <Show when=move || status=="pending">
                                                                <button class="text-blue-600 mr-3" on:click=move |_| {
                                                                    // naive: open a prompt to edit; in real UI, use modal
                                                                    let api = ApiClient::new();
                                                                    let idu = id_sv.get_value();
                                                                    if is_leave {
                                                                        let reason = web_sys::window().and_then(|w| w.prompt_with_message("理由(空で変更なし)").ok().flatten());
                                                                        let payload = json!({"reason": reason});
                                                                        leptos::spawn_local(async move { let _ = api.update_request(&idu, payload).await; });
                                                                    } else {
                                                                        let hours = web_sys::window().and_then(|w| w.prompt_with_message("時間(例:2.0)").ok().flatten()).and_then(|s| s.parse::<f64>().ok());
                                                                        let payload = json!({"planned_hours": hours});
                                                                        leptos::spawn_local(async move { let _ = api.update_request(&idu, payload).await; });
                                                                    }
                                                                }>{"編集"}</button>
                                                                <button class="text-red-600" on:click=move |_| {
                                                                    if web_sys::window().and_then(|w| w.confirm_with_message("取消しますか?").ok()).unwrap_or(false) {
                                                                        let api = ApiClient::new();
                                                                        let idc = id_sv.get_value();
                                                                        leptos::spawn_local(async move { let _ = api.cancel_request(&idc).await; });
                                                                    }
                                                                }>{"取消"}</button>
                                                            </Show>
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
            </div>
        </Layout>
    }
}
