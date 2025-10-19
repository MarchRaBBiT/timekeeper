use chrono::{Datelike, Duration, Months, NaiveDate, Utc};
use leptos::*;

use crate::{
    api::ApiClient,
    components::layout::{ErrorMessage, Layout, SuccessMessage},
    state::attendance::{load_attendance_range, load_today_status, use_attendance},
    utils::trigger_csv_download,
};

#[component]
pub fn AttendancePage() -> impl IntoView {
    let (state, set_state) = use_attendance();

    // 初期表示で当日のステータスを読み込む
    create_effect(move |_| {
        let set_state = set_state.clone();
        spawn_local(async move {
            let _ = load_today_status(set_state).await;
        });
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
        }
    };

    let on_select_current_month = {
        let from_input = from_input.clone();
        let to_input = to_input.clone();
        let set_state = set_state.clone();
        let export_error = export_error.clone();
        let export_success = export_success.clone();
        move |_| {
            export_error.set(None);
            export_success.set(None);
            let today = Utc::now().date_naive();
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

    let status_section = {
        let set_state = set_state.clone();
        move || {
            state
                .get()
                .today_status
                .clone()
                .map(|status| {
                    let (label, color) = match status.status.as_str() {
                        "not_started" => ("未出勤", "bg-gray-100 text-gray-800"),
                        "clocked_in" => ("出勤中", "bg-blue-100 text-blue-800"),
                        "on_break" => ("休憩中", "bg-yellow-100 text-yellow-800"),
                        "clocked_out" => ("退勤済", "bg-green-100 text-green-800"),
                        _ => ("-", "bg-gray-100 text-gray-800"),
                    };

                    let set_state_clock = set_state.clone();
                    let set_state_break_end = set_state.clone();
                    let set_state_break_start = set_state.clone();

                    view! {
                        <div class="rounded-md p-4 border flex items-center justify-between">
                            <span class=format!("inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {}", color)>{label}</span>
                            <div class="space-x-2">
                                <button
                                    class="px-3 py-1 rounded bg-indigo-600 text-white disabled:opacity-50"
                                    disabled={status.status != "not_started"}
                                    on:click=move |_| {
                                        let set_state = set_state_clock.clone();
                                        spawn_local(async move {
                                            let _ = crate::state::attendance::clock_in(set_state.clone()).await;
                                            let _ = load_today_status(set_state).await;
                                        });
                                    }
                                >
                                    {"出勤"}
                                </button>
                                <button
                                    class="px-3 py-1 rounded bg-amber-600 text-white disabled:opacity-50"
                                    disabled={status.status != "clocked_in"}
                                    on:click=move |_| {
                                        if let Some(att_id) = status.attendance_id.clone() {
                                            let set_state = set_state_break_start.clone();
                                            spawn_local(async move {
                                                let api = ApiClient::new();
                                                let _ = api.break_start(&att_id).await;
                                                let _ = load_today_status(set_state).await;
                                            });
                                        }
                                    }
                                >
                                    {"休憩開始"}
                                </button>
                                <button
                                    class="px-3 py-1 rounded bg-amber-700 text-white disabled:opacity-50"
                                    disabled={status.status != "on_break"}
                                    on:click=move |_| {
                                        if let Some(break_id) = status.active_break_id.clone() {
                                            let set_state = set_state_break_end.clone();
                                            spawn_local(async move {
                                                let api = ApiClient::new();
                                                let _ = api.break_end(&break_id).await;
                                                let _ = load_today_status(set_state).await;
                                            });
                                        }
                                    }
                                >
                                    {"休憩終了"}
                                </button>
                                <button
                                    class="px-3 py-1 rounded bg-red-600 text-white disabled:opacity-50"
                                    disabled={status.status == "not_started" || status.status == "clocked_out"}
                                    on:click=move |_| {
                                        let set_state = set_state.clone();
                                        spawn_local(async move {
                                            let _ = crate::state::attendance::clock_out(set_state.clone()).await;
                                            let _ = load_today_status(set_state).await;
                                        });
                                    }
                                >
                                    {"退勤"}
                                </button>
                            </div>
                        </div>
                    }
                })
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
                    {status_section()}
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

                <div class="bg-white shadow overflow-hidden sm:rounded-md">
                    <ul class="divide-y divide-gray-200">
                        <For
                            each=move || state.get().attendance_history.clone()
                            key=|item| item.id.clone()
                            children=move |item| {
                                let id = item.id.clone();
                                let expanded = create_rw_signal(false);
                                let breaks = create_rw_signal(Vec::<crate::api::BreakRecordResponse>::new());
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
