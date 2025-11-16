use crate::api::{
    AdminAttendanceUpsert, AdminBreakItem, AdminHolidayListItem, AdminHolidayListParams, ApiClient,
    CreateHolidayRequest, CreateWeeklyHolidayRequest, HolidayResponse, UserResponse,
    WeeklyHolidayResponse,
};
use crate::components::cards::{
    AdminHolidayListCard, AdminRequestCard, HolidayManagementCard, ManualAttendanceCard,
    MfaResetCard, RequestDetailModal, WeeklyHolidayCard,
};
use crate::components::layout::*;
use crate::state::auth::use_auth;
use crate::utils::time::{now_in_app_tz, today_in_app_tz};
use chrono::{Datelike, NaiveDate, NaiveDateTime};
use leptos::*;
use std::{collections::HashSet, rc::Rc};

fn parse_dt_local(s: &str) -> Option<NaiveDateTime> {
    if s.len() == 16 {
        NaiveDateTime::parse_from_str(&format!("{}:00", s), "%Y-%m-%dT%H:%M:%S").ok()
    } else {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").ok()
    }
}

#[cfg(test)]
mod tests {
    use super::next_allowed_weekly_start;
    use chrono::NaiveDate;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn non_system_admin_must_start_from_tomorrow() {
        let today = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        let expected = NaiveDate::from_ymd_opt(2025, 1, 16).unwrap();
        assert_eq!(next_allowed_weekly_start(today, false), expected);
    }

    #[wasm_bindgen_test]
    fn system_admin_can_start_today() {
        let today = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        assert_eq!(next_allowed_weekly_start(today, true), today);
    }

    #[wasm_bindgen_test]
    fn weekday_label_returns_expected_code() {
        assert_eq!(super::weekday_label(0), "月");
        assert_eq!(super::weekday_label(6), "日");
        assert_eq!(super::weekday_label(9), "-");
    }
}

fn next_allowed_weekly_start(today: NaiveDate, is_system_admin: bool) -> NaiveDate {
    if is_system_admin {
        today
    } else {
        today.succ_opt().unwrap_or(today)
    }
}

fn weekday_label(idx: i16) -> &'static str {
    match idx {
        0 => "月",
        1 => "火",
        2 => "水",
        3 => "木",
        4 => "金",
        5 => "土",
        6 => "日",
        _ => "-",
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

    let load_list: Rc<dyn Fn()> = {
        let status = status.clone();
        let user_id = user_id.clone();
        let page = page.clone();
        let per_page = per_page.clone();
        let list = list.clone();
        Rc::new(move || {
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
        })
    };

    let load_list_store = store_value(load_list.clone());

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

    let open_modal: Rc<dyn Fn(String, serde_json::Value)> = {
        let modal_kind = modal_kind.clone();
        let modal_data = modal_data.clone();
        let modal_comment = modal_comment.clone();
        let show_modal = show_modal.clone();
        Rc::new(move |kind: String, data: serde_json::Value| {
            modal_kind.set(kind);
            modal_data.set(data);
            modal_comment.set(String::new());
            show_modal.set(true);
        })
    };

    let on_approve: Rc<dyn Fn(leptos::ev::MouseEvent)> = {
        let modal_data = modal_data.clone();
        let modal_comment = modal_comment.clone();
        let show_modal = show_modal.clone();
        let load_list_cb = load_list.clone();
        Rc::new(move |_| {
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
            load_list_cb();
        })
    };

    let on_reject: Rc<dyn Fn(leptos::ev::MouseEvent)> = {
        let modal_data = modal_data.clone();
        let modal_comment = modal_comment.clone();
        let show_modal = show_modal.clone();
        let load_list_cb = load_list.clone();
        Rc::new(move |_| {
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
            load_list_cb();
        })
    };

    // Attendance upsert form
    let att_user = create_rw_signal(String::new());
    let att_date = create_rw_signal(String::new());
    let att_in = create_rw_signal(String::new()); // datetime-local
    let att_out = create_rw_signal(String::new()); // datetime-local
    let breaks = create_rw_signal(Vec::<(String, String)>::new());

    let add_break: Rc<dyn Fn(leptos::ev::MouseEvent)> = {
        let breaks = breaks.clone();
        Rc::new(move |_| {
            let mut current = breaks.get();
            current.push((String::new(), String::new()));
            breaks.set(current);
        })
    };

    let on_submit_att: Rc<dyn Fn(leptos::ev::SubmitEvent)> = {
        let att_user = att_user.clone();
        let att_date = att_date.clone();
        let att_in = att_in.clone();
        let att_out = att_out.clone();
        let breaks_signal = breaks.clone();
        Rc::new(move |ev: leptos::ev::SubmitEvent| {
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
            for (s, e) in breaks_signal.get() {
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
        })
    };

    // Force end break quick tool
    let feb_id = create_rw_signal(String::new());
    let on_force_end: Rc<dyn Fn(leptos::ev::MouseEvent)> = {
        let feb_id = feb_id.clone();
        Rc::new(move |_| {
            let id = feb_id.get();
            if id.is_empty() {
                return;
            }
            leptos::spawn_local(async move {
                let api = ApiClient::new();
                let _ = api.admin_force_end_break(&id).await;
            });
        })
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

    let on_reset_mfa: Rc<dyn Fn()> = {
        let selected_mfa_user = selected_mfa_user.clone();
        let mfa_reset_message = mfa_reset_message.clone();
        let mfa_users_signal = mfa_users.clone();
        Rc::new(move || {
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
        })
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
    let google_year_input = create_rw_signal(now_in_app_tz().year().to_string());
    let google_holidays = create_rw_signal(Vec::<CreateHolidayRequest>::new());
    let admin_holiday_items = create_rw_signal(Vec::<AdminHolidayListItem>::new());
    let admin_holiday_total = create_rw_signal(0i64);
    let admin_holiday_page = create_rw_signal(1u32);
    let admin_holiday_per_page = create_rw_signal(10u32);
    let admin_holiday_type = create_rw_signal(String::from("all"));
    let admin_holiday_from = create_rw_signal(String::new());
    let admin_holiday_to = create_rw_signal(String::new());
    let admin_holiday_loading = create_rw_signal(false);
    let admin_holiday_error = create_rw_signal(None::<String>);
    let google_loading = create_rw_signal(false);
    let google_error = create_rw_signal(None::<String>);
    let weekly_start_min = create_memo({
        let system_admin_allowed = system_admin_allowed.clone();
        move |_| {
            next_allowed_weekly_start(today_in_app_tz(), system_admin_allowed.get())
                .format("%Y-%m-%d")
                .to_string()
        }
    });
    let weekly_holidays = create_rw_signal(Vec::<WeeklyHolidayResponse>::new());
    let weekly_loading = create_rw_signal(false);
    let weekly_error = create_rw_signal(None::<String>);
    let weekly_message = create_rw_signal(None::<String>);
    let weekly_weekday_input = create_rw_signal(String::from("0"));
    let weekly_starts_on_input = create_rw_signal(String::new());
    let weekly_ends_on_input = create_rw_signal(String::new());
    let initial_weekly_start = next_allowed_weekly_start(
        today_in_app_tz(),
        system_admin_allowed.try_with(|flag| *flag).unwrap_or(false),
    )
    .format("%Y-%m-%d")
    .to_string();
    weekly_starts_on_input.set(initial_weekly_start);

    let refresh_holidays = {
        let holidays = holidays.clone();
        let holidays_loading = holidays_loading.clone();
        let holiday_error = holiday_error.clone();
        move || {
            holidays_loading.set(true);
            holiday_error.set(None);
            spawn_local(async move {
                let api = ApiClient::new();
                let mut params = AdminHolidayListParams::default();
                params.per_page = 100;
                params.kind = Some("public".into());
                let mut combined: Vec<HolidayResponse> = Vec::new();
                let mut next_page = 1u32;
                loop {
                    params.page = next_page;
                    match api.admin_list_holidays(&params).await {
                        Ok(resp) => {
                            let total = resp.total;
                            let mut items = resp.items;
                            let is_empty = items.is_empty();
                            combined.extend(items.drain(..).filter_map(|item| {
                                item.date.map(|d| HolidayResponse {
                                    id: item.id,
                                    holiday_date: d,
                                    name: item.name.unwrap_or_else(|| "-".into()),
                                    description: item.description,
                                })
                            }));

                            if is_empty || total == 0 || combined.len() as i64 >= total {
                                break;
                            }
                            next_page += 1;
                        }
                        Err(err) => {
                            holiday_error.set(Some(err));
                            break;
                        }
                    }
                }
                combined.sort_by_key(|h| h.holiday_date);
                holidays.set(combined);
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

    let refresh_weekly_holidays = {
        let weekly_holidays = weekly_holidays.clone();
        let weekly_loading = weekly_loading.clone();
        let weekly_error = weekly_error.clone();
        move || {
            weekly_loading.set(true);
            weekly_error.set(None);
            let weekly_holidays = weekly_holidays.clone();
            let weekly_loading = weekly_loading.clone();
            let weekly_error = weekly_error.clone();
            spawn_local(async move {
                let api = ApiClient::new();
                match api.admin_list_weekly_holidays().await {
                    Ok(list) => weekly_holidays.set(list),
                    Err(err) => weekly_error.set(Some(err)),
                }
                weekly_loading.set(false);
            });
        }
    };
    let refresh_weekly_holidays = Rc::new(refresh_weekly_holidays);

    {
        let refresh = refresh_weekly_holidays.clone();
        let admin_allowed_for_weekly = admin_allowed.clone();
        create_effect(move |_| {
            if !admin_allowed_for_weekly.get() {
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

    let fetch_admin_holidays = {
        let items = admin_holiday_items.clone();
        let total = admin_holiday_total.clone();
        let loading = admin_holiday_loading.clone();
        let error = admin_holiday_error.clone();
        let page = admin_holiday_page.clone();
        let per_page = admin_holiday_per_page.clone();
        let kind = admin_holiday_type.clone();
        let from = admin_holiday_from.clone();
        let to = admin_holiday_to.clone();
        move || {
            loading.set(true);
            error.set(None);
            let params = AdminHolidayListParams {
                page: page.get(),
                per_page: per_page.get(),
                kind: {
                    let v = kind.get();
                    if v == "all" || v.is_empty() {
                        None
                    } else {
                        Some(v)
                    }
                },
                from: {
                    let val = from.get();
                    if val.trim().is_empty() {
                        None
                    } else {
                        Some(val)
                    }
                },
                to: {
                    let val = to.get();
                    if val.trim().is_empty() {
                        None
                    } else {
                        Some(val)
                    }
                },
            };
            let items_set = items.clone();
            let total_set = total.clone();
            let loading_set = loading.clone();
            let error_set = error.clone();
            leptos::spawn_local(async move {
                let api = ApiClient::new();
                match api.admin_list_holidays(&params).await {
                    Ok(resp) => {
                        items_set.set(resp.items);
                        total_set.set(resp.total);
                    }
                    Err(err) => error_set.set(Some(err)),
                }
                loading_set.set(false);
            });
        }
    };
    let fetch_admin_holidays = Rc::new(fetch_admin_holidays);

    {
        let fetch = fetch_admin_holidays.clone();
        let admin_allowed_for_list = admin_allowed.clone();
        create_effect(move |_| {
            if admin_allowed_for_list.get() {
                fetch();
            }
        });
    }

    let admin_holiday_total_pages = create_memo({
        let total = admin_holiday_total.clone();
        let per_page = admin_holiday_per_page.clone();
        move |_| {
            let total = total.get();
            let per = per_page.get().max(1) as i64;
            ((total + per - 1) / per).max(1) as u32
        }
    });

    let on_create_holiday: Rc<dyn Fn(leptos::ev::SubmitEvent)> = {
        let holiday_date_input = holiday_date_input.clone();
        let holiday_name_input = holiday_name_input.clone();
        let holiday_desc_input = holiday_desc_input.clone();
        let holiday_message = holiday_message.clone();
        let holiday_error = holiday_error.clone();
        let holiday_saving = holiday_saving.clone();
        let holidays = holidays.clone();
        Rc::new(move |ev: leptos::ev::SubmitEvent| {
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
        })
    };

    let import_google_holidays: Rc<dyn Fn(leptos::ev::MouseEvent)> = {
        let google_holidays = google_holidays.clone();
        let holidays = holidays.clone();
        let holiday_error = holiday_error.clone();
        let holiday_message = holiday_message.clone();
        let holiday_saving = holiday_saving.clone();
        Rc::new(move |_| {
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
        })
    };

    let on_create_weekly_holiday: Rc<dyn Fn(leptos::ev::SubmitEvent)> = {
        let weekly_weekday_input = weekly_weekday_input.clone();
        let weekly_starts_on_input = weekly_starts_on_input.clone();
        let weekly_ends_on_input = weekly_ends_on_input.clone();
        let weekly_error = weekly_error.clone();
        let weekly_message = weekly_message.clone();
        let weekly_loading = weekly_loading.clone();
        let weekly_holidays = weekly_holidays.clone();
        let weekly_start_min = weekly_start_min.clone();
        let system_admin_allowed = system_admin_allowed.clone();
        Rc::new(move |ev: leptos::ev::SubmitEvent| {
            ev.prevent_default();
            if weekly_loading.get() {
                return;
            }
            weekly_error.set(None);
            weekly_message.set(None);

            let weekday_value: u8 = match weekly_weekday_input.get().trim().parse::<u8>() {
                Ok(value) if value < 7 => value,
                _ => {
                    weekly_error.set(Some("曜日は 0 (月) 〜 6 (日) で指定してください。".into()));
                    return;
                }
            };

            let start_raw = weekly_starts_on_input.get();
            if start_raw.trim().is_empty() {
                weekly_error.set(Some("適用開始日を入力してください。".into()));
                return;
            }
            let start_date = match NaiveDate::parse_from_str(start_raw.trim(), "%Y-%m-%d") {
                Ok(date) => date,
                Err(_) => {
                    weekly_error.set(Some(
                        "適用開始日は YYYY-MM-DD 形式で入力してください。".into(),
                    ));
                    return;
                }
            };

            if !system_admin_allowed.get()
                && NaiveDate::parse_from_str(&weekly_start_min.get(), "%Y-%m-%d")
                    .map(|min| start_date < min)
                    .unwrap_or(false)
            {
                weekly_error.set(Some(format!(
                    "開始日は {} 以降を指定してください。",
                    weekly_start_min.get()
                )));
                return;
            }

            let end_date = {
                let raw = weekly_ends_on_input.get();
                if raw.trim().is_empty() {
                    None
                } else {
                    match NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d") {
                        Ok(date) => Some(date),
                        Err(_) => {
                            weekly_error.set(Some(
                                "適用終了日は YYYY-MM-DD 形式で入力してください。".into(),
                            ));
                            return;
                        }
                    }
                }
            };

            if let Some(end) = end_date {
                if end < start_date {
                    weekly_error.set(Some("終了日は開始日以降を選択してください。".into()));
                    return;
                }
            }

            weekly_loading.set(true);
            let weekly_error = weekly_error.clone();
            let weekly_message = weekly_message.clone();
            let weekly_loading = weekly_loading.clone();
            let weekly_holidays_signal = weekly_holidays.clone();
            let weekly_starts_on_input = weekly_starts_on_input.clone();
            let weekly_ends_on_input = weekly_ends_on_input.clone();
            spawn_local(async move {
                let api = ApiClient::new();
                let payload = CreateWeeklyHolidayRequest {
                    weekday: weekday_value,
                    starts_on: start_date,
                    ends_on: end_date,
                };
                match api.admin_create_weekly_holiday(&payload).await {
                    Ok(created) => {
                        weekly_holidays_signal.update(|list| {
                            list.push(created.clone());
                            list.sort_by_key(|h| (h.weekday, h.starts_on));
                        });
                        weekly_message.set(Some(format!(
                            "{}（{}〜{}）を登録しました。",
                            weekday_label(created.weekday),
                            created.starts_on.format("%Y-%m-%d"),
                            created
                                .ends_on
                                .map(|d| d.format("%Y-%m-%d").to_string())
                                .unwrap_or_else(|| "未設定".into())
                        )));
                        weekly_starts_on_input.set(start_date.format("%Y-%m-%d").to_string());
                        weekly_ends_on_input.set(String::new());
                    }
                    Err(err) => weekly_error.set(Some(err)),
                }
                weekly_loading.set(false);
            });
        })
    };

    let on_reset_mfa_store = store_value(on_reset_mfa);
    let add_break_store = store_value(add_break);
    let on_submit_att_store = store_value(on_submit_att);
    let on_force_end_store = store_value(on_force_end);
    let on_reject_store = store_value(on_reject);
    let on_approve_store = store_value(on_approve);

    view! {
        <Layout>
            <Show
                when=move || admin_allowed.get()
                fallback=move || view! {
                    <div class="space-y-6">
                        <div class="bg-white shadow rounded-lg p-6">
                            <p class="text-sm text-gray-700">
                                {"管理者権限が必要です。システム管理者にご連絡ください。"}
                            </p>
                        </div>
                    </div>
                }.into_view()
            >
                <div class="space-y-6">
                    <div>
                        <h1 class="text-2xl font-bold text-gray-900">{"管理者画面"}</h1>
                        <p class="mt-1 text-sm text-gray-600">{"申請の承認/却下、勤怠の手動登録ができます。"}</p>
                    </div>
                    <WeeklyHolidayCard
                        weekly_weekday_input=weekly_weekday_input
                        weekly_starts_on_input=weekly_starts_on_input
                        weekly_ends_on_input=weekly_ends_on_input
                        weekly_start_min=weekly_start_min
                        weekly_loading=weekly_loading
                        weekly_error=weekly_error
                        weekly_message=weekly_message
                        weekly_holidays=weekly_holidays
                        refresh_weekly_holidays=refresh_weekly_holidays.clone()
                        on_create_weekly_holiday=on_create_weekly_holiday.clone()
                    />

                    <div class="grid grid-cols-1 gap-6 lg:grid-cols-3">
                        <AdminRequestCard
                            status=status
                            user_id=user_id
                            list=list
                            load_list=load_list_store.get_value()
                            open_modal=open_modal.clone()
                        />
                        <Show when=move || system_admin_allowed.get()>
                            <ManualAttendanceCard
                                att_user=att_user
                                att_date=att_date
                                att_in=att_in
                                att_out=att_out
                                breaks=breaks
                                add_break=add_break_store.get_value()
                                on_submit_att=on_submit_att_store.get_value()
                                feb_id=feb_id
                                on_force_end=on_force_end_store.get_value()
                            />
                        </Show>
                    </div>
                    <Show when=move || system_admin_allowed.get()>
                        <MfaResetCard
                            mfa_users=mfa_users.read_only()
                            selected_mfa_user=selected_mfa_user
                            mfa_reset_message=mfa_reset_message
                            on_reset_mfa=on_reset_mfa_store.get_value()
                        />
                    </Show>
                    <HolidayManagementCard
                        holidays=holidays
                        holidays_loading=holidays_loading
                        holiday_saving=holiday_saving
                        holiday_message=holiday_message
                        holiday_error=holiday_error
                        holiday_date_input=holiday_date_input
                        holiday_name_input=holiday_name_input
                        holiday_desc_input=holiday_desc_input
                        holiday_deleting=holiday_deleting
                        refresh_holidays=refresh_holidays.clone()
                        on_create_holiday=on_create_holiday.clone()
                        google_year_input=google_year_input
                        google_holidays=google_holidays
                        google_loading=google_loading
                        google_error=google_error
                        fetch_google_holidays=fetch_google_holidays.clone()
                        import_google_holidays=import_google_holidays.clone()
                    />

                    <RequestDetailModal
                        show_modal=show_modal
                        modal_data=modal_data
                        modal_comment=modal_comment
                        on_reject=on_reject_store.get_value()
                        on_approve=on_approve_store.get_value()
                    />
                    <AdminHolidayListCard
                        admin_holiday_total=admin_holiday_total
                        admin_holiday_total_pages=admin_holiday_total_pages
                        admin_holiday_page=admin_holiday_page
                        admin_holiday_per_page=admin_holiday_per_page
                        admin_holiday_type=admin_holiday_type
                        admin_holiday_from=admin_holiday_from
                        admin_holiday_to=admin_holiday_to
                        admin_holiday_loading=admin_holiday_loading
                        admin_holiday_error=admin_holiday_error
                        admin_holiday_items=admin_holiday_items
                        fetch_admin_holidays=fetch_admin_holidays.clone()
                    />
                </div>
            </Show>
        </Layout>
    }
}
