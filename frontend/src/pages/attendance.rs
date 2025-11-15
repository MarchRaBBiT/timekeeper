use chrono::{Datelike, Duration, Months, NaiveDate};
use leptos::*;
use std::rc::Rc;

use crate::{
    api::{ApiClient, HolidayCalendarEntry},
    components::{
        forms::AttendanceActionButtons,
        layout::{ErrorMessage, Layout, SuccessMessage},
    },
    state::attendance::{
        describe_holiday_reason, load_attendance_range, refresh_today_context, use_attendance,
    },
    utils::{time::today_in_app_tz, trigger_csv_download},
};
use log::error;

#[component]
pub fn AttendancePage() -> impl IntoView {
    let (state, set_state) = use_attendance();
    let holiday_entries = create_rw_signal(Vec::<HolidayCalendarEntry>::new());
    let holiday_entries_loading = create_rw_signal(false);
    let holiday_entries_error = create_rw_signal(None::<String>);
    let initial_today = today_in_app_tz();
    let active_holiday_period = create_rw_signal((initial_today.year(), initial_today.month()));
    let fetch_month_holidays = Rc::new({
        let holiday_entries = holiday_entries.clone();
        let holiday_entries_loading = holiday_entries_loading.clone();
        let holiday_entries_error = holiday_entries_error.clone();
        let active_holiday_period = active_holiday_period.clone();
        move |year: i32, month: u32| {
            holiday_entries_loading.set(true);
            holiday_entries_error.set(None);
            let holiday_entries = holiday_entries.clone();
            let holiday_entries_loading = holiday_entries_loading.clone();
            let holiday_entries_error = holiday_entries_error.clone();
            let active_holiday_period = active_holiday_period.clone();
            spawn_local(async move {
                let api = ApiClient::new();
                match api.get_monthly_holidays(year, month).await {
                    Ok(entries) => {
                        holiday_entries.set(entries);
                        active_holiday_period.set((year, month));
                    }
                    Err(err) => holiday_entries_error.set(Some(err)),
                }
                holiday_entries_loading.set(false);
            });
        }
    });
    let fetch_month_for_effect = fetch_month_holidays.clone();
    create_effect(move |_| {
        let set_state_for_status = set_state.clone();
        let fetch_month = fetch_month_for_effect.clone();
        spawn_local(async move {
            if let Err(err) = refresh_today_context(set_state_for_status).await {
                error!("Failed to refresh attendance context: {}", err);
            }
        });
        fetch_month(initial_today.year(), initial_today.month());
    });

    let from_input = create_rw_signal(String::new());
    let to_input = create_rw_signal(String::new());
    let export_error = create_rw_signal(Option::<String>::None);
    let export_success = create_rw_signal(Option::<String>::None);
    let exporting = create_rw_signal(false);

    let on_load_range = {
        let set_state = set_state.clone();
        let from_input = from_input.clone();
        let to_input = to_input.clone();
        let fetch_month_holidays = fetch_month_holidays.clone();
        move |_| {
            let from = if from_input.get().is_empty() {
                None
            } else {
                NaiveDate::parse_from_str(&from_input.get(), "%Y-%m-%d").ok()
            };
            let to = if to_input.get().is_empty() {
                None
            } else {
                NaiveDate::parse_from_str(&to_input.get(), "%Y-%m-%d").ok()
            };
            spawn_local(async move {
                let _ = load_attendance_range(set_state, from, to).await;
            });
            if let Some(date) = from {
                fetch_month_holidays(date.year(), date.month());
            }
        }
    };

    let on_select_current_month = {
        let from_input = from_input.clone();
        let to_input = to_input.clone();
        let set_state = set_state.clone();
        let export_error = export_error.clone();
        let export_success = export_success.clone();
        let fetch_month_holidays = fetch_month_holidays.clone();
        move |_| {
            export_error.set(None);
            export_success.set(None);
            let today = today_in_app_tz();
            let Some(first_day) = NaiveDate::from_ymd_opt(today.year(), today.month(), 1) else {
                return;
            };
            let Some(next_month) = first_day.checked_add_months(Months::new(1)) else {
                return;
            };
            let Some(last_day) = next_month.checked_sub_signed(Duration::days(1)) else {
                return;
            };
            from_input.set(first_day.format("%Y-%m-%d").to_string());
            to_input.set(last_day.format("%Y-%m-%d").to_string());
            let set_state_for_async = set_state.clone();
            spawn_local(async move {
                let _ = load_attendance_range(set_state_for_async, Some(first_day), Some(last_day))
                    .await;
            });
            fetch_month_holidays(first_day.year(), first_day.month());
        }
    };

    let on_export_csv = {
        let from_input = from_input.clone();
        let to_input = to_input.clone();
        let export_error = export_error.clone();
        let export_success = export_success.clone();
        let exporting = exporting.clone();
        move |_| {
            export_error.set(None);
            export_success.set(None);

            let from_val = from_input.get();
            let to_val = to_input.get();
            let mut validation_error: Option<String> = None;

            let from_date = if from_val.is_empty() {
                None
            } else {
                match NaiveDate::parse_from_str(&from_val, "%Y-%m-%d") {
                    Ok(date) => Some(date),
                    Err(_) => {
                        validation_error =
                            Some("開始日は YYYY-MM-DD 形式で入力してください。".into());
                        None
                    }
                }
            };

            let to_date = if to_val.is_empty() {
                None
            } else {
                match NaiveDate::parse_from_str(&to_val, "%Y-%m-%d") {
                    Ok(date) => Some(date),
                    Err(_) => {
                        validation_error =
                            Some("終了日は YYYY-MM-DD 形式で入力してください。".into());
                        None
                    }
                }
            };

            if validation_error.is_none() {
                if let (Some(from_date), Some(to_date)) = (from_date, to_date) {
                    if from_date > to_date {
                        validation_error =
                            Some("開始日は終了日以前の日付を指定してください。".into());
                    }
                }
            }

            if let Some(message) = validation_error {
                export_error.set(Some(message));
                return;
            }

            exporting.set(true);
            let from_owned = if from_val.is_empty() {
                None
            } else {
                Some(from_val)
            };
            let to_owned = if to_val.is_empty() {
                None
            } else {
                Some(to_val)
            };

            spawn_local(async move {
                let api = ApiClient::new();
                match api
                    .export_my_attendance_filtered(from_owned.as_deref(), to_owned.as_deref())
                    .await
                {
                    Ok(payload) => {
                        let filename = payload
                            .get("filename")
                            .and_then(|v| v.as_str())
                            .unwrap_or("my_attendance.csv");
                        let csv = payload
                            .get("csv_data")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        match trigger_csv_download(filename, csv) {
                            Ok(_) => {
                                export_success
                                    .set(Some(format!("{filename} をダウンロードしました")));
                            }
                            Err(err) => {
                                export_error
                                    .set(Some(format!("CSVのダウンロードに失敗しました: {err}")));
                            }
                        }
                    }
                    Err(err) => {
                        export_error.set(Some(err));
                    }
                }
                exporting.set(false);
            });
        }
    };

    view! {
        <Layout>
            <div class="space-y-6">
                <div>
                    <h1 class="text-2xl font-bold text-gray-900">{"勤怠管理"}</h1>
                    <p class="mt-1 text-sm text-gray-600">{"当日のステータスを確認できます。"}</p>
                </div>

                <Show when=move || state.get().today_status.is_some()>
                    <div class="rounded-md p-4 border bg-white shadow-sm">
                        <AttendanceActionButtons
                            attendance_state=state
                            set_attendance_state=set_state
                        />
                    </div>
                </Show>

                <div class="bg-white shadow rounded-lg p-4 flex items-end space-x-3">
                    <div>
                        <label class="block text-sm font-medium text-gray-700">{"開始日"}</label>
                        <input
                            type="date"
                            class="mt-1 block w-full border-gray-300 rounded-md shadow-sm"
                            prop:value={move || from_input.get()}
                            on:input=move |ev| from_input.set(event_target_value(&ev))
                        />
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700">{"終了日"}</label>
                        <input
                            type="date"
                            class="mt-1 block w-full border-gray-300 rounded-md shadow-sm"
                            prop:value={move || to_input.get()}
                            on:input=move |ev| to_input.set(event_target_value(&ev))
                        />
                    </div>
                    <button
                        class="px-4 py-2 bg-gray-200 text-gray-800 rounded hover:bg-gray-300"
                        on:click=on_select_current_month
                    >
                        {"今月"}
                    </button>
                    <button class="px-4 py-2 bg-blue-600 text-white rounded" on:click=on_load_range>
                        {"読込"}
                    </button>
                    <button
                        class="px-4 py-2 bg-indigo-600 text-white rounded disabled:opacity-50"
                        disabled={move || exporting.get()}
                        on:click=on_export_csv
                    >
                        {move || if exporting.get() { "CSV生成中..." } else { "CSVダウンロード" }}
                    </button>
                </div>

                <Show when=move || export_error.get().is_some()>
                    <ErrorMessage message={export_error.get().unwrap_or_default()} />
                </Show>

                <Show when=move || export_success.get().is_some()>
                    <SuccessMessage message={export_success.get().unwrap_or_default()} />
                </Show>
                {move || {
                    state
                        .get()
                        .last_refresh_error
                        .clone()
                        .map(|msg| view! { <ErrorMessage message={msg}/> }.into_view())
                        .unwrap_or_else(|| view! {}.into_view())
                }}

                <div class="bg-white shadow rounded-lg p-4 space-y-3">
                    <div class="flex flex-col gap-1 md:flex-row md:items-center md:justify-between">
                        <div>
                            <h3 class="text-base font-semibold text-gray-900">{"今月の休日"}</h3>
                            <p class="text-sm text-gray-600">
                                {move || {
                                    let (year, month) = active_holiday_period.get();
                                    format!("{year}年{month}月に予定されている休日日程です。")
                                }}
                            </p>
                        </div>
                        <button
                            class="px-3 py-1 text-sm rounded border text-gray-700 hover:bg-gray-50 disabled:opacity-50"
                            disabled={move || holiday_entries_loading.get()}
                            on:click={
                                let fetch = fetch_month_holidays.clone();
                                move |_| {
                                    let (year, month) = active_holiday_period.get();
                                    fetch(year, month);
                                }
                            }
                        >
                            {"再取得"}
                        </button>
                    </div>
                    <Show when=move || holiday_entries_error.get().is_some()>
                        <ErrorMessage message={holiday_entries_error.get().unwrap_or_default()} />
                    </Show>
                    <Show when=move || holiday_entries_loading.get()>
                        <p class="text-sm text-gray-500">{"休日情報を読み込み中です..."}</p>
                    </Show>
                    <Show
                        when=move || !holiday_entries_loading.get() && holiday_entries.get().is_empty()
                    >
                        <p class="text-sm text-gray-500">{"登録済みの休日はまだありません。管理者に確認してください。"}
                        </p>
                    </Show>
                    <Show
                        when=move || !holiday_entries_loading.get() && !holiday_entries.get().is_empty()
                    >
                        <ul class="space-y-2">
                            <For
                                each=move || holiday_entries.get()
                                key=|entry| (entry.date, entry.reason.clone())
                                children=move |entry: HolidayCalendarEntry| {
                                    let label = describe_holiday_reason(entry.reason.as_str());
                                    view! {
                                        <li class="flex items-center justify-between text-sm text-gray-800">
                                            <span>{entry.date.format("%Y-%m-%d").to_string()}</span>
                                            <span class="px-2 py-0.5 rounded-full text-xs bg-blue-50 text-blue-800">
                                                {label}
                                            </span>
                                        </li>
                                    }
                                }
                            />
                        </ul>
                    </Show>
                </div>

                <div class="bg-white shadow overflow-hidden sm:rounded-md">
                    <ul class="divide-y divide-gray-200">
                        <For
                            each=move || state.get().attendance_history.clone()
                            key=|item| item.id.clone()
                            children=move |item| {
                                let id = item.id.clone();
                                let expanded = create_rw_signal(false);
                                let breaks = create_rw_signal(Vec::<crate::api::BreakRecordResponse>::new());
                                let date_for_lookup = item.date;
                                let monthly_holidays = holiday_entries.clone();
                                let day_holiday_reason = create_memo(move |_| {
                                    monthly_holidays
                                        .get()
                                        .iter()
                                        .find(|entry| entry.date == date_for_lookup)
                                        .map(|entry| entry.reason.clone())
                                });
                                let toggle = {
                                    let expanded = expanded.clone();
                                    let breaks = breaks.clone();
                                    move |_| {
                                        let now_expanded = !expanded.get();
                                        expanded.set(now_expanded);
                                        if now_expanded {
                                            let attendance_id = id.clone();
                                            let breaks = breaks.clone();
                                            spawn_local(async move {
                                                let api = ApiClient::new();
                                                if let Ok(list) = api.get_breaks_by_attendance(&attendance_id).await {
                                                    breaks.set(list);
                                                }
                                            });
                                        }
                                    }
                                };

                                view! {
                                    <li class="px-6 py-4">
                                        <div class="flex items-center justify-between">
                                            <div>
                                                <div class="text-sm font-medium text-gray-900">
                                                    {format!("{}", item.date)}
                                                </div>
                                                <Show when=move || day_holiday_reason.get().is_some()>
                                                    <span class="inline-flex items-center mt-1 px-2 py-0.5 rounded-full text-xs bg-amber-50 text-amber-800">
                                                        {move || {
                                                            day_holiday_reason
                                                                .get()
                                                                .as_ref()
                                                                .map(|code| describe_holiday_reason(code.trim()).to_string())
                                                                .unwrap_or_default()
                                                        }}
                                                    </span>
                                                </Show>
                                                <div class="text-sm text-gray-500">
                                                    {match (item.clock_in_time, item.clock_out_time) {
                                                        (Some(start), Some(end)) => format!("{} - {}", start.format("%H:%M"), end.format("%H:%M")),
                                                        (Some(start), None) => format!("{} -", start.format("%H:%M")),
                                                        _ => "-".to_string(),
                                                    }}
                                                </div>
                                            </div>
                                            <div class="flex items-center space-x-4">
                                                <div class="text-sm font-medium text-gray-900">
                                                    {item.total_work_hours.map(|h| format!("{:.1}時間", h)).unwrap_or_default()}
                                                </div>
                                                <button class="text-blue-600" on:click=toggle.clone()>
                                                    {move || if expanded.get() { "閉じる" } else { "詳細" }}
                                                </button>
                                            </div>
                                        </div>

                                        <Show when=move || expanded.get()>
                                            <div class="mt-3 text-sm text-gray-700">
                                                <div class="font-semibold mb-1">{"休憩"}</div>
                                                <ul class="list-disc ml-6 space-y-1">
                                                    <For
                                                        each=move || breaks.get()
                                                        key=|b| b.id.clone()
                                                        children=move |b| {
                                                            view! {
                                                                <li>
                                                                    {format!(
                                                                        "{} - {} ({}分)",
                                                                        b.break_start_time.format("%H:%M"),
                                                                        b.break_end_time
                                                                            .map(|t| t.format("%H:%M").to_string())
                                                                            .unwrap_or_else(|| "-".into()),
                                                                        b.duration_minutes.unwrap_or(0)
                                                                    )}
                                                                </li>
                                                            }
                                                        }
                                                    />
                                                </ul>
                                            </div>
                                        </Show>
                                    </li>
                                }
                            }
                        />
                    </ul>
                </div>
            </div>
        </Layout>
    }
}
