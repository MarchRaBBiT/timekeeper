use crate::api::{AdminAttendanceUpsert, AdminBreakItem, ApiClient};
use chrono::{NaiveDate, NaiveDateTime};
use leptos::*;

fn parse_dt_local(s: &str) -> Option<NaiveDateTime> {
    if s.len() == 16 {
        NaiveDateTime::parse_from_str(&format!("{}:00", s), "%Y-%m-%dT%H:%M:%S").ok()
    } else {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").ok()
    }
}

#[component]
pub fn AdminAttendanceToolsSection(system_admin_allowed: Memo<bool>) -> impl IntoView {
    let att_user = create_rw_signal(String::new());
    let att_date = create_rw_signal(String::new());
    let att_in = create_rw_signal(String::new());
    let att_out = create_rw_signal(String::new());
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
        <Show when=move || system_admin_allowed.get()>
            <div class="bg-white shadow rounded-lg p-6">
                <h3 class="text-lg font-medium text-gray-900 mb-4">{"各種打刻登録（アップサート）"}</h3>
                <form class="space-y-3" on:submit=on_submit_att>
                    <input placeholder="User ID" class="w-full border rounded px-2 py-1" on:input=move |ev| att_user.set(event_target_value(&ev)) />
                    <input type="date" class="w-full border rounded px-2 py-1" on:input=move |ev| att_date.set(event_target_value(&ev)) />
                    <input type="datetime-local" class="w-full border rounded px-2 py-1" on:input=move |ev| att_in.set(event_target_value(&ev)) />
                    <input type="datetime-local" class="w-full border rounded px-2 py-1" on:input=move |ev| att_out.set(event_target_value(&ev)) />
                    <div>
                        <div class="flex items-center justify-between mb-1">
                            <span class="text-sm text-gray-700">{"休憩（任意）"}</span>
                            <button type="button" class="text-blue-600 text-sm" on:click=add_break>{"枠を追加"}</button>
                        </div>
                        <For each=move || breaks.get() key=|pair| pair.clone() children=move |(s0, e0)| {
                            let s = create_rw_signal(s0);
                            let e = create_rw_signal(e0);
                            view! {
                                <div class="flex space-x-2 mb-2">
                                    <input type="datetime-local" class="border rounded px-2 py-1 w-full" prop:value=s on:input=move |ev| s.set(event_target_value(&ev)) />
                                    <input type="datetime-local" class="border rounded px-2 py-1 w-full" prop:value=e on:input=move |ev| e.set(event_target_value(&ev)) />
                                </div>
                            }
                        } />
                    </div>
                    <button type="submit" class="w-full bg-green-600 text-white rounded py-2">{"登録"}</button>
                </form>
                <div class="mt-4">
                    <h4 class="text-sm font-medium text-gray-900 mb-2">{"休憩の強制終了"}</h4>
                    <div class="flex space-x-2">
                        <input placeholder="Break ID" class="border rounded px-2 py-1 w-full" on:input=move |ev| feb_id.set(event_target_value(&ev)) />
                        <button class="px-3 py-1 bg-amber-600 text-white rounded" on:click=on_force_end>{"即時終了"}</button>
                    </div>
                </div>
            </div>
        </Show>
    }
}
