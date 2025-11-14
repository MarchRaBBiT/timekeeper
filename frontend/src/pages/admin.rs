use crate::api::{
    AdminAttendanceUpsert, AdminBreakItem, ApiClient, CreateHolidayRequest, HolidayResponse,
    UserResponse,
};
use crate::components::layout::*;
use crate::state::auth::use_auth;
use chrono::{Datelike, NaiveDate, NaiveDateTime, Utc};
use leptos::*;
use serde_json::json;
use std::{collections::HashSet, rc::Rc};
use web_sys::HtmlSelectElement;

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

    let (auth, _set_auth) = use_auth();
    let auth_for_admin = auth.clone();
    let admin_allowed = create_memo(move |_| {
        auth_for_admin
            .get()
            .user
            .as_ref()
            .map(|user| user.is_system_admin || user.role.eq_ignore_ascii_case("admin"))
            .unwrap_or(false)
    });
    let auth_for_system = auth.clone();
    let system_admin_allowed = create_memo(move |_| {
        auth_for_system
            .get()
            .user
            .as_ref()
            .map(|user| user.is_system_admin)
            .unwrap_or(false)
    });

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

    // System admin MFA reset tool
    let mfa_users = create_rw_signal(Vec::<UserResponse>::new());
    let selected_mfa_user = create_rw_signal(String::new());
    let mfa_reset_message = create_rw_signal(None::<String>);

    {
        let mfa_users = mfa_users.clone();
        spawn_local(async move {
            let api = ApiClient::new();
            if let Ok(users) = api.get_users().await {
                mfa_users.set(users);
            }
        });
    }

    let on_reset_mfa = {
        let selected_mfa_user = selected_mfa_user.clone();
        let mfa_reset_message = mfa_reset_message.clone();
        let mfa_users_signal = mfa_users.clone();
        move |_| {
            let target = selected_mfa_user.get();
            if target.is_empty() {
                mfa_reset_message.set(Some("ユーザーを選択してください".into()));
                return;
            }
            let msg = mfa_reset_message.clone();
            let user_id = target.clone();
            let display_name = mfa_users_signal
                .get()
                .into_iter()
                .find(|u| u.id == user_id)
                .map(|u| format!("{} ({})", u.full_name, u.username))
                .unwrap_or_else(|| user_id.clone());
            spawn_local(async move {
                let api = ApiClient::new();
                match api.admin_reset_mfa(&user_id).await {
                    Ok(_) => msg.set(Some(format!("{} のMFAをリセットしました。", display_name))),
                    Err(err) => msg.set(Some(format!("MFAリセットに失敗しました: {}", err))),
                }
            });
        }
    };

    let holidays = create_rw_signal(Vec::<HolidayResponse>::new());
    let holidays_loading = create_rw_signal(false);
    let holiday_saving = create_rw_signal(false);
    let holiday_message = create_rw_signal(None::<String>);
    let holiday_error = create_rw_signal(None::<String>);
    let holiday_date_input = create_rw_signal(String::new());
    let holiday_name_input = create_rw_signal(String::new());
    let holiday_desc_input = create_rw_signal(String::new());
    let holiday_deleting = create_rw_signal(None::<String>);
    let google_year_input = create_rw_signal(Utc::now().year().to_string());
    let google_holidays = create_rw_signal(Vec::<CreateHolidayRequest>::new());
    let google_loading = create_rw_signal(false);
    let google_error = create_rw_signal(None::<String>);

    let refresh_holidays = {
        let holidays = holidays.clone();
        let holidays_loading = holidays_loading.clone();
        let holiday_error = holiday_error.clone();
        move || {
            holidays_loading.set(true);
            holiday_error.set(None);
            spawn_local(async move {
                let api = ApiClient::new();
                match api.admin_list_holidays().await {
                    Ok(mut list) => {
                        list.sort_by_key(|h| h.holiday_date);
                        holidays.set(list);
                    }
                    Err(err) => holiday_error.set(Some(err)),
                }
                holidays_loading.set(false);
            });
        }
    };
    let refresh_holidays = Rc::new(refresh_holidays);
    {
        let refresh = refresh_holidays.clone();
        let admin_allowed_for_holidays = admin_allowed.clone();
        create_effect(move |_| {
            if !admin_allowed_for_holidays.get() {
                return;
            }
            refresh();
        });
    }

    let fetch_google_holidays = {
        let google_year_input = google_year_input.clone();
        let google_loading = google_loading.clone();
        let google_error = google_error.clone();
        let google_holidays = google_holidays.clone();
        move |_| {
            google_loading.set(true);
            google_error.set(None);
            let google_year_input = google_year_input.get();
            spawn_local(async move {
                let api = ApiClient::new();
                let year = google_year_input.trim().parse::<i32>().ok();
                match api.admin_fetch_google_holidays(year).await {
                    Ok(mut list) => {
                        list.sort_by_key(|h| (h.holiday_date, h.name.clone()));
                        google_holidays.set(list);
                    }
                    Err(err) => google_error.set(Some(err)),
                }
                google_loading.set(false);
            });
        }
    };
    let fetch_google_holidays = Rc::new(fetch_google_holidays);

    let on_create_holiday = {
        let holiday_date_input = holiday_date_input.clone();
        let holiday_name_input = holiday_name_input.clone();
        let holiday_desc_input = holiday_desc_input.clone();
        let holiday_message = holiday_message.clone();
        let holiday_error = holiday_error.clone();
        let holiday_saving = holiday_saving.clone();
        let holidays = holidays.clone();
        move |ev: leptos::ev::SubmitEvent| {
            ev.prevent_default();
            let date_raw = holiday_date_input.get();
            let name_raw = holiday_name_input.get();
            let desc_raw = holiday_desc_input.get();
            if date_raw.trim().is_empty() {
                holiday_error.set(Some("休日の日付を入力してください".into()));
                return;
            }
            let parsed_date = match NaiveDate::parse_from_str(&date_raw, "%Y-%m-%d") {
                Ok(date) => date,
                Err(_) => {
                    holiday_error.set(Some("日付の形式が正しくありません (YYYY-MM-DD)".into()));
                    return;
                }
            };
            let trimmed_name = name_raw.trim().to_string();
            if trimmed_name.is_empty() {
                holiday_error.set(Some("休日名を入力してください".into()));
                return;
            }
            let description = {
                let trimmed = desc_raw.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            };
            holiday_saving.set(true);
            holiday_error.set(None);
            holiday_message.set(None);
            let holiday_message = holiday_message.clone();
            let holiday_error = holiday_error.clone();
            let holiday_saving = holiday_saving.clone();
            let holidays_signal = holidays.clone();
            let holiday_date_input = holiday_date_input.clone();
            let holiday_name_input = holiday_name_input.clone();
            let holiday_desc_input = holiday_desc_input.clone();
            spawn_local(async move {
                let api = ApiClient::new();
                let payload = CreateHolidayRequest {
                    holiday_date: parsed_date,
                    name: trimmed_name.clone(),
                    description,
                };
                match api.admin_create_holiday(&payload).await {
                    Ok(created) => {
                        holidays_signal.update(|list| {
                            list.push(created.clone());
                            list.sort_by_key(|h| h.holiday_date);
                        });
                        holiday_message.set(Some(format!(
                            "{} ({}) を登録しました。",
                            created.name,
                            created.holiday_date.format("%Y-%m-%d")
                        )));
                        holiday_date_input.set(String::new());
                        holiday_name_input.set(String::new());
                        holiday_desc_input.set(String::new());
                    }
                    Err(err) => holiday_error.set(Some(err)),
                }
                holiday_saving.set(false);
            });
        }
    };

    let import_google_holidays = {
        let google_holidays = google_holidays.clone();
        let holidays = holidays.clone();
        let holiday_error = holiday_error.clone();
        let holiday_message = holiday_message.clone();
        let holiday_saving = holiday_saving.clone();
        move |_| {
            let existing_dates: HashSet<NaiveDate> =
                holidays.get().into_iter().map(|h| h.holiday_date).collect();
            let to_create: Vec<CreateHolidayRequest> = google_holidays
                .get()
                .into_iter()
                .filter(|candidate| !existing_dates.contains(&candidate.holiday_date))
                .collect();
            if to_create.is_empty() {
                holiday_message.set(Some("追加対象の休日はありません。".into()));
                return;
            }
            holiday_error.set(None);
            holiday_message.set(None);
            holiday_saving.set(true);
            let holidays_signal = holidays.clone();
            let holiday_message_signal = holiday_message.clone();
            let holiday_error_signal = holiday_error.clone();
            spawn_local(async move {
                let api = ApiClient::new();
                let mut success_count = 0;
                for payload in to_create {
                    match api.admin_create_holiday(&payload).await {
                        Ok(created) => {
                            holidays_signal.update(|list| {
                                list.push(created.clone());
                                list.sort_by_key(|h| h.holiday_date);
                            });
                            success_count += 1;
                        }
                        Err(err) => {
                            holiday_error_signal.set(Some(format!(
                                "{} の登録に失敗しました: {}",
                                payload.holiday_date.format("%Y-%m-%d"),
                                err
                            )));
                            break;
                        }
                    }
                }
                if success_count > 0 {
                    holiday_message_signal
                        .set(Some(format!("{}件の休日を追加しました。", success_count)));
                }
                holiday_saving.set(false);
            });
        }
    };

    view! {
        <Layout>
            <Show
                when=move || admin_allowed.get()
                fallback=move || {
                    view! {
                        <div class="space-y-6">
                            <div class="bg-white shadow rounded-lg p-6">
                                <p class="text-sm text-gray-700">
                                    {"管理者権限が必要です。システム管理者にご連絡ください。"}
                                </p>
                            </div>
                        </div>
                    }
                }
            >
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

                <Show when=move || system_admin_allowed.get()>
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
                </Show>
                <Show when=move || system_admin_allowed.get()>
                <div class="bg-white shadow rounded-lg p-4 space-y-4">
                    <h2 class="text-lg font-semibold text-gray-900">{"MFA リセット (システム管理者専用)"}</h2>
                    <p class="text-sm text-gray-600">
                        {"選択したユーザーの MFA 設定をリセットし、次回ログイン時に再登録を求めます。"}
                    </p>
                    <div>
                        <label for="mfa-reset-user" class="block text-sm font-medium text-gray-700">
                            {"対象ユーザー"}
                        </label>
                        <select
                            id="mfa-reset-user"
                            class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500"
                            on:change=move |ev| {
                                let target = event_target::<HtmlSelectElement>(&ev);
                                selected_mfa_user.set(target.value());
                            }
                        >
                            <option value="">{ "ユーザーを選択してください" }</option>
                            {move || {
                                mfa_users
                                    .get()
                                    .into_iter()
                                    .map(|user| {
                                        view! {
                                            <option value={user.id.clone()}>
                                                {format!("{} ({})", user.full_name, user.username)}
                                            </option>
                                        }
                                    })
                                    .collect_view()
                            }}
                        </select>
                    </div>
                    <div>
                        <button
                            on:click=on_reset_mfa
                            class="px-4 py-2 rounded-md bg-red-600 text-white hover:bg-red-700 disabled:opacity-50"
                        >
                            {"MFAをリセット"}
                        </button>
                    </div>
                    {move || {
                        mfa_reset_message
                            .get()
                            .map(|msg| view! { <p class="text-sm text-gray-700">{msg}</p> }.into_view())
                            .unwrap_or_else(|| view! {}.into_view())
                    }}
                </div>
                </Show>
                <div class="bg-white shadow rounded-lg p-6 space-y-4">
                    <div class="flex flex-col gap-1 md:flex-row md:items-center md:justify-between">
                        <div>
                            <h2 class="text-lg font-semibold text-gray-900">{"休日設定"}</h2>
                            <p class="text-sm text-gray-600">
                                {"登録した休日は出勤可否や表示に利用されます。"}
                            </p>
                        </div>
                        <button
                            class="px-3 py-1 rounded border text-sm text-gray-700 hover:bg-gray-50 disabled:opacity-50"
                            disabled={move || holidays_loading.get()}
                            on:click={
                                let refresh = refresh_holidays.clone();
                                move |_| refresh()
                            }
                        >
                            {"再読込"}
                        </button>
                    </div>
                    <form class="grid gap-3 md:grid-cols-3" on:submit=on_create_holiday>
                        <div class="md:col-span-1">
                            <label class="block text-sm font-medium text-gray-700">{"日付"}</label>
                            <input
                                type="date"
                                class="mt-1 w-full border rounded px-2 py-1"
                                prop:value={move || holiday_date_input.get()}
                                on:input=move |ev| holiday_date_input.set(event_target_value(&ev))
                            />
                        </div>
                        <div class="md:col-span-1">
                            <label class="block text-sm font-medium text-gray-700">{"休日名"}</label>
                            <input
                                type="text"
                                class="mt-1 w-full border rounded px-2 py-1"
                                placeholder="例: 振替休日"
                                prop:value={move || holiday_name_input.get()}
                                on:input=move |ev| holiday_name_input.set(event_target_value(&ev))
                            />
                        </div>
                        <div class="md:col-span-1">
                            <label class="block text-sm font-medium text-gray-700">{"メモ"}</label>
                            <input
                                type="text"
                                class="mt-1 w-full border rounded px-2 py-1"
                                placeholder="任意"
                                prop:value={move || holiday_desc_input.get()}
                                on:input=move |ev| holiday_desc_input.set(event_target_value(&ev))
                            />
                        </div>
                        <div class="md:col-span-3">
                            <button
                                type="submit"
                                class="w-full md:w-auto px-4 py-2 rounded bg-green-600 text-white hover:bg-green-700 disabled:opacity-50"
                                disabled={move || holiday_saving.get()}
                            >
                                {move || if holiday_saving.get() { "登録中..." } else { "休日を追加" }}
                            </button>
                        </div>
                    </form>
                    {move || {
                        holiday_message
                            .get()
                            .map(|msg| view! { <p class="text-sm text-green-600">{msg}</p> }.into_view())
                            .unwrap_or_else(|| view! {}.into_view())
                    }}
                    {move || {
                        holiday_error
                            .get()
                            .map(|msg| view! { <p class="text-sm text-red-600">{msg}</p> }.into_view())
                            .unwrap_or_else(|| view! {}.into_view())
                    }}
                    <Show when=move || holidays_loading.get()>
                        <p class="text-sm text-gray-500">{"読込中..."}</p>
                    </Show>
                    <Show when=move || !holidays_loading.get() && holidays.get().is_empty()>
                        <p class="text-sm text-gray-500">{"登録された休日はまだありません。先に追加してください。"}</p>
                    </Show>
                    <Show when=move || !holidays_loading.get() && !holidays.get().is_empty()>
                        <div class="overflow-x-auto">
                            <table class="min-w-full divide-y divide-gray-200">
                                <thead class="bg-gray-50">
                                    <tr>
                                        <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"日付"}</th>
                                        <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"名称"}</th>
                                        <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"説明"}</th>
                                        <th class="px-4 py-2"/>
                                    </tr>
                                </thead>
                                <tbody class="bg-white divide-y divide-gray-200">
                                    <For
                                        each=move || holidays.get()
                                        key=|item| item.id.clone()
                                        children=move |holiday: HolidayResponse| {
                                            let id = holiday.id.clone();
                                            let label = format!(
                                                "{} {}",
                                                holiday.holiday_date.format("%Y-%m-%d"),
                                                holiday.name
                                            );
                                            let desc = holiday.description.clone().unwrap_or_else(|| "なし".into());
                                            let holidays_signal = holidays.clone();
                                            let holiday_message_signal = holiday_message.clone();
                                            let holiday_error_signal = holiday_error.clone();
                                            let holiday_deleting_signal = holiday_deleting.clone();
                                            let id_for_disable = id.clone();
                                            view! {
                                                <tr>
                                                    <td class="px-4 py-2 text-sm text-gray-900">{holiday.holiday_date.format("%Y-%m-%d").to_string()}</td>
                                                    <td class="px-4 py-2 text-sm text-gray-900">{holiday.name.clone()}</td>
                                                    <td class="px-4 py-2 text-sm text-gray-600">{desc}</td>
                                                    <td class="px-4 py-2 text-right">
                                                        <button
                                                            class="text-sm text-red-600 hover:text-red-700 disabled:opacity-50"
                                                            disabled={move || {
                                                                match holiday_deleting_signal.get() {
                                                                    Some(current) => current == id_for_disable,
                                                                    None => false,
                                                                }
                                                            }}
                                                            on:click=move |_| {
                                                                holiday_error_signal.set(None);
                                                                holiday_message_signal.set(None);
                                                                holiday_deleting_signal.set(Some(id.clone()));
                                                                let id_for_task = id.clone();
                                                                let label_for_task = label.clone();
                                                                let holidays_signal = holidays_signal.clone();
                                                                let holiday_message_signal = holiday_message_signal.clone();
                                                                let holiday_error_signal = holiday_error_signal.clone();
                                                                let holiday_deleting_signal = holiday_deleting_signal.clone();
                                                                spawn_local(async move {
                                                                    let api = ApiClient::new();
                                                                    match api.admin_delete_holiday(&id_for_task).await {
                                                                        Ok(_) => {
                                                                            holidays_signal.update(|list| list.retain(|h| h.id != id_for_task));
                                                                            holiday_message_signal.set(Some(format!("{} を削除しました。", label_for_task)));
                                                                        }
                                                                        Err(err) => holiday_error_signal.set(Some(err)),
                                                                    }
                                                                    holiday_deleting_signal.set(None);
                                                                });
                                                            }
                                                        >
                                                            {"削除"}
                                                        </button>
                                                    </td>
                                                </tr>
                                            }
                                        }
                                    />
                                </tbody>
                            </table>
                        </div>
                    </Show>
                    <div class="border-t border-dashed border-gray-200 pt-4 space-y-3">
                        <div class="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
                            <div>
                                <h3 class="text-base font-semibold text-gray-900">
                                    {"Google カレンダーの日本の祝日から取り込み"}
                                </h3>
                                <p class="text-sm text-gray-600">
                                    {"Google公開カレンダーから祝日を取得し、未登録のものを一括で追加できます。"}
                                </p>
                            </div>
                            <div class="flex flex-col gap-2 sm:flex-row sm:items-center">
                                <input
                                    type="number"
                                    min="2000"
                                    class="w-full sm:w-32 border rounded px-2 py-1"
                                    placeholder="年 (任意)"
                                    prop:value={move || google_year_input.get()}
                                    on:input=move |ev| google_year_input.set(event_target_value(&ev))
                                />
                                <button
                                    class="px-3 py-1 rounded border text-sm text-gray-700 hover:bg-gray-50 disabled:opacity-50"
                                    disabled={move || google_loading.get()}
                                    on:click={
                                        let fetch_action = fetch_google_holidays.clone();
                                        move |ev| fetch_action(ev)
                                    }
                                >
                                    {move || if google_loading.get() { "取得中..." } else { "祝日を取得" }}
                                </button>
                            </div>
                        </div>
                        {move || {
                            google_error
                                .get()
                                .map(|msg| view! { <p class="text-sm text-red-600">{msg}</p> }.into_view())
                                .unwrap_or_else(|| view! {}.into_view())
                        }}
                        <Show when=move || google_loading.get()>
                            <p class="text-sm text-gray-500">{"Google カレンダーから取得中..."}</p>
                        </Show>
                        <Show when=move || !google_loading.get() && google_holidays.get().is_empty()>
                            <p class="text-sm text-gray-500">
                                {"取得済みの祝日はここに表示されます。年を指定して「祝日を取得」を押してください。"}
                            </p>
                        </Show>
                        <Show when=move || !google_loading.get() && !google_holidays.get().is_empty()>
                            <div class="space-y-2">
                                <div class="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
                                    <p class="text-sm text-gray-700">
                                        {"未登録の祝日は「未登録」ラベルが表示されます。"}
                                    </p>
                                    <button
                                        class="px-4 py-2 rounded bg-blue-600 text-white hover:bg-blue-700 disabled:opacity-50"
                                        disabled={move || {
                                            if holiday_saving.get() {
                                                true
                                            } else {
                                                let existing = holidays.get();
                                                !google_holidays
                                                    .get()
                                                    .iter()
                                                    .any(|g| existing.iter().all(|h| h.holiday_date != g.holiday_date))
                                            }
                                        }}
                                        on:click=import_google_holidays
                                    >
                                        {move || if holiday_saving.get() { "取り込み中..." } else { "未登録の祝日をすべて追加" }}
                                    </button>
                                </div>
                                <div class="overflow-x-auto">
                                    <table class="min-w-full divide-y divide-gray-200 text-sm">
                                        <thead class="bg-gray-50">
                                            <tr>
                                                <th class="px-3 py-2 text-left font-medium text-gray-500 uppercase tracking-wider">{"日付"}</th>
                                                <th class="px-3 py-2 text-left font-medium text-gray-500 uppercase tracking-wider">{"名称"}</th>
                                                <th class="px-3 py-2 text-left font-medium text-gray-500 uppercase tracking-wider">{"状態"}</th>
                                            </tr>
                                        </thead>
                                        <tbody class="bg-white divide-y divide-gray-200">
                                            <For
                                                each=move || google_holidays.get()
                                                key=|item| format!("{}-{}", item.holiday_date, item.name)
                                                children=move |candidate: CreateHolidayRequest| {
                                                    let holidays_state = holidays.clone();
                                                    let duplicate = holidays_state
                                                        .get()
                                                        .iter()
                                                        .any(|existing| existing.holiday_date == candidate.holiday_date);
                                                    view! {
                                                        <tr>
                                                            <td class="px-3 py-2 text-gray-900">{candidate.holiday_date.format("%Y-%m-%d").to_string()}</td>
                                                            <td class="px-3 py-2 text-gray-900">{candidate.name.clone()}</td>
                                                            <td class="px-3 py-2">
                                                                {if duplicate {
                                                                    view! { <span class="inline-flex items-center px-2 py-0.5 text-xs rounded-full bg-gray-100 text-gray-600">{"登録済"}</span> }.into_view()
                                                                } else {
                                                                    view! { <span class="inline-flex items-center px-2 py-0.5 text-xs rounded-full bg-green-100 text-green-600">{"未登録"}</span> }.into_view()
                                                                }}
                                                            </td>
                                                        </tr>
                                                    }
                                                }
                                            />
                                        </tbody>
                                    </table>
                                </div>
                            </div>
                        </Show>
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
            </Show>
        </Layout>
    }
}
