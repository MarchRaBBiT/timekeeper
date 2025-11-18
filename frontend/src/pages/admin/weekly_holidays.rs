use crate::api::{ApiClient, CreateWeeklyHolidayRequest, WeeklyHolidayResponse};
use crate::components::layout::{ErrorMessage, SuccessMessage};
use crate::utils::time::today_in_app_tz;
use chrono::NaiveDate;
use leptos::*;
use std::rc::Rc;

#[component]
pub fn WeeklyHolidaySection(
    admin_allowed: Memo<bool>,
    system_admin_allowed: Memo<bool>,
) -> impl IntoView {
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
    let initial_weekly_start = next_allowed_weekly_start(
        today_in_app_tz(),
        system_admin_allowed.try_with(|flag| *flag).unwrap_or(false),
    )
    .format("%Y-%m-%d")
    .to_string();
    let weekly_starts_on_input = create_rw_signal(initial_weekly_start);
    let weekly_ends_on_input = create_rw_signal(String::new());

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

    let on_create_weekly_holiday = {
        let weekly_weekday_input = weekly_weekday_input.clone();
        let weekly_starts_on_input = weekly_starts_on_input.clone();
        let weekly_ends_on_input = weekly_ends_on_input.clone();
        let weekly_error = weekly_error.clone();
        let weekly_message = weekly_message.clone();
        let weekly_loading = weekly_loading.clone();
        let weekly_holidays = weekly_holidays.clone();
        let weekly_start_min = weekly_start_min.clone();
        let system_admin_allowed = system_admin_allowed.clone();
        move |ev: leptos::ev::SubmitEvent| {
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
        }
    };

    view! {
        <div class="bg-white shadow rounded-lg p-6 space-y-4">
            <div class="flex flex-col gap-1 md:flex-row md:items-center md:justify-between">
                <div>
                    <h2 class="text-lg font-semibold text-gray-900">{"定休曜日"}</h2>
                    <p class="text-sm text-gray-600">
                        {"曜日ベースの休日を設定します。一般管理者は翌日以降のみ登録できます。"}
                    </p>
                </div>
                <button
                    class="px-3 py-1 rounded border text-sm text-gray-700 hover:bg-gray-50 disabled:opacity-50"
                    disabled={move || weekly_loading.get()}
                    on:click={
                        let refresh = refresh_weekly_holidays.clone();
                        move |_| refresh()
                    }
                >
                    {"再取得"}
                </button>
            </div>
            <form class="grid gap-3 md:grid-cols-3" on:submit=on_create_weekly_holiday>
                <div class="md:col-span-1">
                    <label class="block text-sm font-medium text-gray-700">{"曜日"}</label>
                    <select
                        class="mt-1 w-full border rounded px-2 py-1"
                        prop:value={move || weekly_weekday_input.get()}
                        on:change=move |ev| weekly_weekday_input.set(event_target_value(&ev))
                    >
                        <option value="0">{"月曜 (0)"}</option>
                        <option value="1">{"火曜 (1)"}</option>
                        <option value="2">{"水曜 (2)"}</option>
                        <option value="3">{"木曜 (3)"}</option>
                        <option value="4">{"金曜 (4)"}</option>
                        <option value="5">{"土曜 (5)"}</option>
                        <option value="6">{"日曜 (6)"}</option>
                    </select>
                </div>
                <div class="md:col-span-1">
                    <label class="block text-sm font-medium text-gray-700">{"適用開始日"}</label>
                    <input
                        type="date"
                        class="mt-1 w-full border rounded px-2 py-1"
                        prop:value={move || weekly_starts_on_input.get()}
                        prop:min={move || weekly_start_min.get()}
                        on:input=move |ev| weekly_starts_on_input.set(event_target_value(&ev))
                    />
                    <p class="text-xs text-gray-500 mt-1">
                        {move || {
                            if system_admin_allowed.get() {
                                "システム管理者は当日から登録できます。"
                            } else {
                                "一般管理者は翌日以降を指定してください。"
                            }
                        }}
                    </p>
                </div>
                <div class="md:col-span-1">
                    <label class="block text-sm font-medium text-gray-700">{"適用終了日 (任意)"}</label>
                    <input
                        type="date"
                        class="mt-1 w-full border rounded px-2 py-1"
                        prop:value={move || weekly_ends_on_input.get()}
                        on:input=move |ev| weekly_ends_on_input.set(event_target_value(&ev))
                    />
                </div>
                <div class="md:col-span-3">
                    <button
                        type="submit"
                        class="w-full md:w-auto px-4 py-2 rounded bg-blue-600 text-white hover:bg-blue-700 disabled:opacity-50"
                        disabled={move || weekly_loading.get()}
                    >
                        {move || if weekly_loading.get() { "登録中..." } else { "定休を追加" }}
                    </button>
                </div>
            </form>
            <Show when=move || weekly_error.get().is_some()>
                <ErrorMessage message={weekly_error.get().unwrap_or_default()} />
            </Show>
            <Show when=move || weekly_message.get().is_some()>
                <SuccessMessage message={weekly_message.get().unwrap_or_default()} />
            </Show>
            <Show when=move || weekly_loading.get() && weekly_holidays.get().is_empty()>
                <p class="text-sm text-gray-500">{"定休曜日を読み込み中です..."}</p>
            </Show>
            <Show when=move || !weekly_loading.get() && weekly_holidays.get().is_empty()>
                <p class="text-sm text-gray-500">
                    {"登録された定休曜日はまだありません。先に追加してください。"}
                </p>
            </Show>
            <Show when=move || !weekly_loading.get() && !weekly_holidays.get().is_empty()>
                <div class="overflow-x-auto">
                    <table class="min-w-full divide-y divide-gray-200 text-sm">
                        <thead class="bg-gray-50">
                            <tr>
                                <th class="px-4 py-2 text-left font-medium text-gray-500 uppercase tracking-wider">{"曜日"}</th>
                                <th class="px-4 py-2 text-left font-medium text-gray-500 uppercase tracking-wider">{"適用期間"}</th>
                                <th class="px-4 py-2 text-left font-medium text-gray-500 uppercase tracking-wider">{"履歴"}</th>
                            </tr>
                        </thead>
                        <tbody class="bg-white divide-y divide-gray-200">
                            <For
                                each=move || weekly_holidays.get()
                                key=|item| item.id.clone()
                                children=move |item: WeeklyHolidayResponse| {
                                    view! {
                                        <tr>
                                            <td class="px-4 py-2 text-gray-900">{weekday_label(item.weekday)}</td>
                                            <td class="px-4 py-2 text-gray-900">
                                                {format!(
                                                    "{} 〜 {}",
                                                    item.starts_on.format("%Y-%m-%d"),
                                                    item
                                                        .ends_on
                                                        .map(|d| d.format("%Y-%m-%d").to_string())
                                                        .unwrap_or_else(|| "未設定".into())
                                                )}
                                            </td>
                                            <td class="px-4 py-2 text-gray-600 text-sm">
                                                {format!(
                                                    "{} 〜 {}",
                                                    item.enforced_from.format("%Y-%m-%d"),
                                                    item
                                                        .enforced_to
                                                        .map(|d| d.format("%Y-%m-%d").to_string())
                                                        .unwrap_or_else(|| "継続中".into())
                                                )}
                                            </td>
                                        </tr>
                                    }
                                }
                            />
                        </tbody>
                    </table>
                </div>
            </Show>
        </div>
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
