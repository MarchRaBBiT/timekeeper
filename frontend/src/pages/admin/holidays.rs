use crate::api::{ApiClient, CreateHolidayRequest, HolidayResponse};
use crate::utils::time::now_in_app_tz;
use chrono::{Datelike, NaiveDate};
use leptos::*;
use std::collections::HashSet;
use std::rc::Rc;

#[component]
pub fn HolidayManagementSection(admin_allowed: Memo<bool>) -> impl IntoView {
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
    let google_loading = create_rw_signal(false);
    let google_error = create_rw_signal(None::<String>);

    let refresh_holidays = Rc::new({
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
    });

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

    let fetch_google_holidays = Rc::new({
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
    });

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
                    holiday_error.set(Some("日付の書式が正しくありません (YYYY-MM-DD)".into()));
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
        <div class="bg-white shadow rounded-lg p-6 space-y-4">
            <div class="flex flex-col gap-1 md:flex-row md:items-center md:justify-between">
                <div>
                    <h2 class="text-lg font-semibold text-gray-900">{"休日設定"}</h2>
                    <p class="text-sm text-gray-600">
                        {"登録済みの休日は申請モジュール内で利用されます。"}
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
                    {"再取得"}
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
                        placeholder="例: 建国記念の日"
                        prop:value={move || holiday_name_input.get()}
                        on:input=move |ev| holiday_name_input.set(event_target_value(&ev))
                    />
                </div>
                <div class="md:col-span-1">
                    <label class="block text-sm font-medium text-gray-700">{"備考"}</label>
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
                <p class="text-sm text-gray-500">{"読み込み中..."}</p>
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
                                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"備考"}</th>
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
                            {"Google カレンダーの祝日候補を取得"}
                        </h3>
                        <p class="text-sm text-gray-600">
                            {"Googleの公開カレンダーから祝日を取得し、未登録のもののみまとめて追加できます。"}
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
                        {"取得済みの祝日はまだ表示されません。年を指定して「祝日を取得」を押してください。"}
                    </p>
                </Show>
                <Show when=move || !google_loading.get() && !google_holidays.get().is_empty()>
                    <div class="space-y-2">
                        <div class="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
                            <p class="text-sm text-gray-700">
                                {"未登録の祝日は「未登録」タグが付きます。"}
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
                                {move || if holiday_saving.get() { "処理中..." } else { "未登録の祝日をまとめて追加" }}
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
    }
}
