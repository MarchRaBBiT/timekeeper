use chrono::{Datelike, Timelike};
use leptos::*;
use std::rc::Rc;
use web_sys::HtmlSelectElement;

use crate::api::{
    AdminHolidayListItem, ApiClient, AttendanceSummary, CreateHolidayRequest, HolidayResponse,
    WeeklyHolidayResponse, UserResponse,
};
use crate::components::layout::{ErrorMessage, SuccessMessage};
use crate::state::attendance::{refresh_today_context, AttendanceState};
use crate::utils::time::{now_in_app_tz, today_in_app_tz};
use serde_json::{self, json};

#[component]
pub fn AttendanceCard(
    attendance_state: ReadSignal<AttendanceState>,
    set_attendance_state: WriteSignal<AttendanceState>,
) -> impl IntoView {
    {
        let set_state = set_attendance_state;
        create_effect(move |_| {
            spawn_local(async move {
                if let Err(err) = refresh_today_context(set_state).await {
                    log::error!("Failed to refresh attendance context: {}", err);
                }
            });
        });
    }

    let today = today_in_app_tz();

    let clock_in = move || {
        attendance_state
            .get()
            .today_status
            .as_ref()
            .and_then(|s| s.clock_in_time)
            .map(|t| format!("{:02}:{:02}", t.hour(), t.minute()))
            .unwrap_or_else(|| "-".into())
    };
    let clock_out = move || {
        attendance_state
            .get()
            .today_status
            .as_ref()
            .and_then(|s| s.clock_out_time)
            .map(|t| format!("{:02}:{:02}", t.hour(), t.minute()))
            .unwrap_or_else(|| "-".into())
    };
    let total_hours = move || {
        attendance_state
            .get()
            .attendance_history
            .iter()
            .find(|a| a.date == today)
            .and_then(|a| a.total_work_hours)
            .map(|h| format!("{:.2}時間", h))
            .unwrap_or_else(|| "-".into())
    };
    let break_minutes = move || {
        let mins: i32 = attendance_state
            .get()
            .attendance_history
            .iter()
            .find(|a| a.date == today)
            .map(|a| {
                a.break_records
                    .iter()
                    .map(|b| b.duration_minutes.unwrap_or(0))
                    .sum()
            })
            .unwrap_or(0);
        if mins > 0 {
            format!("{} 分", mins)
        } else {
            "-".into()
        }
    };

    view! {
        <div class="bg-white overflow-hidden shadow rounded-lg">
            <div class="px-4 py-5 sm:p-6">
                <h3 class="text-lg leading-6 font-medium text-gray-900">
                    {format!("今日の勤怠 ({:04}-{:02}-{:02})", today.year(), today.month(), today.day())}
                </h3>
                <div class="mt-5">
                    <div class="grid grid-cols-2 gap-4">
                        <div>
                            <dt class="text-sm font-medium text-gray-500">{"出勤時間"}</dt>
                            <dd class="mt-1 text-sm text-gray-900">{clock_in}</dd>
                        </div>
                        <div>
                            <dt class="text-sm font-medium text-gray-500">{"退勤時間"}</dt>
                            <dd class="mt-1 text-sm text-gray-900">{clock_out}</dd>
                        </div>
                        <div>
                            <dt class="text-sm font-medium text-gray-500">{"稼働時間"}</dt>
                            <dd class="mt-1 text-sm text-gray-900">{total_hours}</dd>
                        </div>
                        <div>
                            <dt class="text-sm font-medium text-gray-500">{"休憩合計"}</dt>
                            <dd class="mt-1 text-sm text-gray-900">{break_minutes}</dd>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn SummaryCard() -> impl IntoView {
    let (summary, set_summary) = create_signal::<Option<AttendanceSummary>>(None);
    spawn_local(async move {
        let api = ApiClient::new();
        let now = now_in_app_tz();
        let y = now.year();
        let m = now.month() as u32;
        if let Ok(s) = api.get_my_summary(Some(y), Some(m)).await {
            set_summary.set(Some(s));
        }
    });

    view! {
        <div class="bg-white overflow-hidden shadow rounded-lg">
            <div class="px-4 py-5 sm:p-6">
                <h3 class="text-lg leading-6 font-medium text-gray-900">{"今月のサマリー"}</h3>
                <div class="mt-5">
                    <div class="grid grid-cols-3 gap-4">
                        <div>
                            <dt class="text-sm font-medium text-gray-500">{"総稼働時間"}</dt>
                            <dd class="mt-1 text-2xl font-semibold text-gray-900">
                                {move || summary.get().as_ref().map(|s| format!("{:.2}時間", s.total_work_hours)).unwrap_or_else(|| "-".into())}
                            </dd>
                        </div>
                        <div>
                            <dt class="text-sm font-medium text-gray-500">{"稼働日数"}</dt>
                            <dd class="mt-1 text-2xl font-semibold text-gray-900">
                                {move || summary.get().as_ref().map(|s| format!("{} 日", s.total_work_days)).unwrap_or_else(|| "-".into())}
                            </dd>
                        </div>
                        <div>
                            <dt class="text-sm font-medium text-gray-500">{"平均稼働時間"}</dt>
                            <dd class="mt-1 text-2xl font-semibold text-gray-900">
                                {move || summary.get().as_ref().map(|s| format!("{:.2}時間", s.average_daily_hours)).unwrap_or_else(|| "-".into())}
                            </dd>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn RequestCard() -> impl IntoView {
    let (reqs, set_reqs) = create_signal::<Option<serde_json::Value>>(None);
    spawn_local(async move {
        let api = ApiClient::new();
        if let Ok(v) = api.get_my_requests().await {
            set_reqs.set(Some(v));
        }
    });

    let count = move |kind: &str, status: &str| -> i32 {
        let v = match reqs.get() {
            Some(v) => v,
            None => return 0,
        };
        v.get(kind)
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter(|item| item.get("status").and_then(|s| s.as_str()) == Some(status))
                    .count() as i32
            })
            .unwrap_or(0)
    };

    view! {
        <div class="bg-white overflow-hidden shadow rounded-lg">
            <div class="px-4 py-5 sm:p-6">
                <h3 class="text-lg leading-6 font-medium text-gray-900">{"申請状況"}</h3>
                <div class="mt-5 space-y-2">
                    <div class="text-sm text-gray-700">{"休暇申請: 保留="}{move || count("leave_requests","pending")} {" 件 / 承認="}{move || count("leave_requests","approved")} {" 件 / 却下="}{move || count("leave_requests","rejected")} {" 件"}</div>
                    <div class="text-sm text-gray-700">{"残業申請: 保留="}{move || count("overtime_requests","pending")} {" 件 / 承認="}{move || count("overtime_requests","approved")} {" 件 / 却下="}{move || count("overtime_requests","rejected")} {" 件"}</div>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn UserCard() -> impl IntoView {
    use crate::state::auth::use_auth;
    let (auth, _) = use_auth();
    view! {
        <div class="bg-white overflow-hidden shadow rounded-lg">
            <div class="px-4 py-5 sm:p-6">
                <h3 class="text-lg leading-6 font-medium text-gray-900">{"ユーザー情報"}</h3>
                <div class="mt-5">
                    <div class="space-y-2">
                        <div>
                            <dt class="text-sm font-medium text-gray-500">{"ユーザー名"}</dt>
                            <dd class="mt-1 text-sm text-gray-900">{move || auth.get().user.as_ref().map(|u| u.username.clone()).unwrap_or_else(|| "-".into())}</dd>
                        </div>
                        <div>
                            <dt class="text-sm font-medium text-gray-500">{"氏名"}</dt>
                            <dd class="mt-1 text-sm text-gray-900">{move || auth.get().user.as_ref().map(|u| u.full_name.clone()).unwrap_or_else(|| "-".into())}</dd>
                        </div>
                        <div>
                            <dt class="text-sm font-medium text-gray-500">{"役割"}</dt>
                            <dd class="mt-1 text-sm text-gray-900">{move || auth.get().user.as_ref().map(|u| u.role.clone()).unwrap_or_else(|| "-".into())}</dd>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}

fn weekday_label_text(idx: i16) -> &'static str {
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

fn describe_admin_holiday(item: &AdminHolidayListItem) -> String {
    match item.kind.as_str() {
        "public" => item.name.clone().unwrap_or_else(|| "登録済み祝日".into()),
        "weekly" => {
            let w = item.weekday.map(weekday_label_text).unwrap_or("-");
            format!("定休 ({})", w)
        }
        "exception" => {
            if item.is_override.unwrap_or(false) {
                format!(
                    "例外: {} を休日化",
                    item.user_id.clone().unwrap_or_else(|| "-".into())
                )
            } else {
                format!(
                    "例外: {} を稼働日化",
                    item.user_id.clone().unwrap_or_else(|| "-".into())
                )
            }
        }
        _ => item.name.clone().unwrap_or_else(|| "-".into()),
    }
}

fn admin_holiday_range(item: &AdminHolidayListItem) -> String {
    let from = item.applies_from.format("%Y-%m-%d").to_string();
    match item.applies_to {
        Some(to) if to != item.applies_from => {
            format!("{} 〜 {}", from, to.format("%Y-%m-%d"))
        }
        _ => from,
    }
}

#[component]
pub fn WeeklyHolidayCard(
    weekly_weekday_input: RwSignal<String>,
    weekly_starts_on_input: RwSignal<String>,
    weekly_ends_on_input: RwSignal<String>,
    weekly_start_min: Memo<String>,
    weekly_loading: RwSignal<bool>,
    weekly_error: RwSignal<Option<String>>,
    weekly_message: RwSignal<Option<String>>,
    weekly_holidays: RwSignal<Vec<WeeklyHolidayResponse>>,
    refresh_weekly_holidays: Rc<dyn Fn()>,
    on_create_weekly_holiday: Rc<dyn Fn(leptos::ev::SubmitEvent)>,
) -> impl IntoView {
    view! {
        <div class="bg-white shadow rounded-lg p-6 space-y-4">
            <div class="flex flex-col gap-1 md:flex-row md:items-center md:justify-between">
                <div>
                    <h2 class="text-lg font-semibold text-gray-900">{"定休管理"}</h2>
                    <p class="text-sm text-gray-600">
                        {"曜日ベースの休暇を設定します。一般管理者は翌日以降のみ登録できます。"}
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
            <form class="grid gap-3 md:grid-cols-3" on:submit={
                let handler = on_create_weekly_holiday.clone();
                move |ev| handler(ev)
            }>
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
                        min={move || weekly_start_min.get()}
                        prop:value={move || weekly_starts_on_input.get()}
                        on:input=move |ev| weekly_starts_on_input.set(event_target_value(&ev))
                    />
                </div>
                <div class="md:col-span-1">
                    <label class="block text-sm font-medium text-gray-700">{"適用終了日（任意）"}</label>
                    <input
                        type="date"
                        class="mt-1 w-full border rounded px-2 py-1"
                        prop:value={move || weekly_ends_on_input.get()}
                        on:input=move |ev| weekly_ends_on_input.set(event_target_value(&ev))
                    />
                </div>
                <div class="md:col-span-3 md:flex md:items-center md:gap-3">
                    <button
                        class="px-4 py-2 rounded bg-blue-600 text-white hover:bg-blue-700 disabled:opacity-50"
                        type="submit"
                        disabled={move || weekly_loading.get()}
                    >
                        {move || if weekly_loading.get() { "登録中..." } else { "定休を追加" }}
                    </button>
                    <p class="text-xs text-gray-500 mt-2 md:mt-0">
                        {move || format!("システム管理者は当日登録可 / 一般管理者は {} 以降のみ", weekly_start_min.get())}
                    </p>
                </div>
            </form>
            <Show when=move || weekly_error.get().is_some()>
                {move || view! { <ErrorMessage message=weekly_error.get().unwrap_or_else(|| "-".into())/> }}
            </Show>
            <Show when=move || weekly_message.get().is_some()>
                {move || view! { <SuccessMessage message=weekly_message.get().unwrap_or_else(|| "-".into())/> }}
            </Show>
            <Show when=move || weekly_loading.get()>
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
                                <th class="px-4 py-2 text-left font-medium text-gray-500 uppercase tracking-wider">{"登録履歴"}</th>
                            </tr>
                        </thead>
                        <tbody class="bg-white divide-y divide-gray-200">
                            <For
                                each=move || weekly_holidays.get()
                                key=|item| item.id.clone()
                                children=move |item: WeeklyHolidayResponse| {
                                    view! {
                                        <tr>
                                            <td class="px-4 py-2 text-gray-900">{weekday_label_text(item.weekday)}</td>
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
                                            <td class="px-4 py-2 text-gray-600">
                                                {format!(
                                                    "{} 〜 {}",
                                                    item.enforced_from.format("%Y-%m-%d"),
                                                    item
                                                        .enforced_to
                                                        .map(|d| d.format("%Y-%m-%d").to_string())
                                                        .unwrap_or_else(|| "更新中".into())
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

#[component]
pub fn MfaResetCard(
    mfa_users: ReadSignal<Vec<UserResponse>>,
    selected_mfa_user: RwSignal<String>,
    mfa_reset_message: RwSignal<Option<String>>,
    on_reset_mfa: Rc<dyn Fn()>,
) -> impl IntoView {
    view! {
        <div class="bg-white shadow rounded-lg p-4 space-y-4">
            <h2 class="text-lg font-semibold text-gray-900">{"MFA リセット（システム管理者専用）"}</h2>
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
                    on:click=move |_| on_reset_mfa()
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
    }
}

#[component]
pub fn HolidayManagementCard(
    holidays: RwSignal<Vec<HolidayResponse>>,
    holidays_loading: RwSignal<bool>,
    holiday_saving: RwSignal<bool>,
    holiday_message: RwSignal<Option<String>>,
    holiday_error: RwSignal<Option<String>>,
    holiday_date_input: RwSignal<String>,
    holiday_name_input: RwSignal<String>,
    holiday_desc_input: RwSignal<String>,
    holiday_deleting: RwSignal<Option<String>>,
    refresh_holidays: Rc<dyn Fn()>,
    on_create_holiday: Rc<dyn Fn(leptos::ev::SubmitEvent)>,
    google_year_input: RwSignal<String>,
    google_holidays: RwSignal<Vec<CreateHolidayRequest>>,
    google_loading: RwSignal<bool>,
    google_error: RwSignal<Option<String>>,
    fetch_google_holidays: Rc<dyn Fn(leptos::ev::MouseEvent)>,
    import_google_holidays: Rc<dyn Fn(leptos::ev::MouseEvent)>,
) -> impl IntoView {
    view! {
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
            <form class="grid gap-3 md:grid-cols-3" on:submit={
                let handler = on_create_holiday.clone();
                move |ev| handler(ev)
            }>
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
                {holiday_table_view(
                    holidays,
                    holiday_message,
                    holiday_error,
                    holiday_deleting,
                )}
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
                                on:click={
                                    let import_action = import_google_holidays.clone();
                                    move |ev| import_action(ev)
                                }
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
    }
}

fn holiday_table_view(
    holidays: RwSignal<Vec<HolidayResponse>>,
    holiday_message: RwSignal<Option<String>>,
    holiday_error: RwSignal<Option<String>>,
    holiday_deleting: RwSignal<Option<String>>,
) -> impl IntoView {
    view! {
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
                            let id_to_delete = id.clone();
                            let label_to_delete = label.clone();
                            view! {
                                <tr>
                                    <td class="px-4 py-2 text-sm text-gray-900">{holiday.holiday_date.format("%Y-%m-%d").to_string()}</td>
                                    <td class="px-4 py-2 text-sm text-gray-900">{holiday.name.clone()}</td>
                                    <td class="px-4 py-2 text-sm text-gray-600">{desc}</td>
                                    <td class="px-4 py-2 text-right">
                                <button
                                    class="text-sm text-red-600 hover:text-red-700 disabled:opacity-50"
                                    disabled={move || holiday_deleting_signal.get().as_deref() == Some(&id_for_disable)}
                                    on:click=delete_holiday_handler(
                                        id_to_delete.clone(),
                                        label_to_delete.clone(),
                                        holidays_signal.clone(),
                                        holiday_message_signal.clone(),
                                        holiday_error_signal.clone(),
                                        holiday_deleting_signal.clone(),
                                    )
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
    }
}

#[component]
pub fn AdminRequestCard(
    status: RwSignal<String>,
    user_id: RwSignal<String>,
    list: RwSignal<serde_json::Value>,
    load_list: Rc<dyn Fn()>,
    open_modal: Rc<dyn Fn(String, serde_json::Value)>,
) -> impl IntoView {
    view! {
        <div class="bg-white shadow rounded-lg p-6 lg:col-span-2">
            <h3 class="text-lg font-medium text-gray-900 mb-4">{"申請一覧"}</h3>
            <div class="flex space-x-3 mb-4">
                <select
                    class="border-gray-300 rounded-md"
                    on:change={
                        let load = load_list.clone();
                        move |ev| {
                            status.set(event_target_value(&ev));
                            load();
                        }
                    }
                >
                    <option value="">{ "すべて" }</option>
                    <option value="pending">{ "承認待ち" }</option>
                    <option value="approved">{ "承認済" }</option>
                    <option value="rejected">{ "却下" }</option>
                    <option value="cancelled">{ "取消" }</option>
                </select>
                <input
                    placeholder="User ID"
                    class="border rounded-md px-2"
                    on:input=move |ev| user_id.set(event_target_value(&ev))
                />
                <button
                    class="px-3 py-1 bg-blue-600 text-white rounded"
                    on:click={
                        let load = load_list.clone();
                        move |_| load()
                    }
                >
                    {"検索"}
                </button>
            </div>
            <div class="overflow-x-auto">
                <table class="min-w-full divide-y divide-gray-200">
                    <thead class="bg-gray-50">
                        <tr>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"種別"}</th>
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
                                if let Some(arr) = leaves.as_array() {
                                    for r in arr {
                                        rows.push(json!({"kind":"leave","data": r}));
                                    }
                                }
                                if let Some(arr) = ots.as_array() {
                                    for r in arr {
                                        rows.push(json!({"kind":"overtime","data": r}));
                                    }
                                }
                                view! {
                                    <>{
                                        rows.into_iter().map(|row| {
                                            let kind = row.get("kind").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                            let data = row.get("data").cloned().unwrap_or(json!({}));
                                            let _id = data.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                            let statusv = data.get("status").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                            let user = data.get("user_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                            let target = if kind == "leave" {
                                                format!(
                                                    "{} - {}",
                                                    data.get("start_date").and_then(|v| v.as_str()).unwrap_or(""),
                                                    data.get("end_date").and_then(|v| v.as_str()).unwrap_or("")
                                                )
                                            } else {
                                                data.get("date").and_then(|v| v.as_str()).unwrap_or("").to_string()
                                            };
                                            let open_modal = open_modal.clone();
                                            view! {
                                                <tr>
                                                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{if kind == "leave" { "休暇" } else { "残業" }}</td>
                                                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-600">{target}</td>
                                                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-600">{user}</td>
                                                    <td class="px-6 py-4 whitespace-nowrap">
                                                        <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-gray-100 text-gray-800">
                                                            {statusv.clone()}
                                                        </span>
                                                    </td>
                                                    <td class="px-6 py-4 text-right text-sm font-medium">
                                                        <button
                                                            class="text-indigo-600 hover:text-indigo-900"
                                                            on:click=move |_| open_modal(kind.clone(), data.clone())
                                                        >
                                                            {"詳細"}
                                                        </button>
                                                    </td>
                                                </tr>
                                            }
                                        }).collect_view()
                                    }</>
                                }
                            }
                        </Show>
                    </tbody>
                </table>
            </div>
        </div>
    }
}

#[component]
pub fn ManualAttendanceCard(
    att_user: RwSignal<String>,
    att_date: RwSignal<String>,
    att_in: RwSignal<String>,
    att_out: RwSignal<String>,
    breaks: RwSignal<Vec<(String, String)>>,
    add_break: Rc<dyn Fn(leptos::ev::MouseEvent)>,
    on_submit_att: Rc<dyn Fn(leptos::ev::SubmitEvent)>,
    feb_id: RwSignal<String>,
    on_force_end: Rc<dyn Fn(leptos::ev::MouseEvent)>,
) -> impl IntoView {
    view! {
        <div class="bg-white shadow rounded-lg p-6">
            <h3 class="text-lg font-medium text-gray-900 mb-4">{"勤怠の手動登録"}</h3>
            <form
                class="space-y-3"
                on:submit={
                    let handler = on_submit_att.clone();
                    move |ev| handler(ev)
                }
            >
                <input
                    placeholder="User ID"
                    class="w-full border rounded px-2 py-1"
                    on:input=move |ev| att_user.set(event_target_value(&ev))
                />
                <input
                    type="date"
                    class="w-full border rounded px-2 py-1"
                    on:input=move |ev| att_date.set(event_target_value(&ev))
                />
                <input
                    type="datetime-local"
                    class="w-full border rounded px-2 py-1"
                    on:input=move |ev| att_in.set(event_target_value(&ev))
                />
                <input
                    type="datetime-local"
                    class="w-full border rounded px-2 py-1"
                    on:input=move |ev| att_out.set(event_target_value(&ev))
                />
                <div>
                    <div class="flex items-center justify-between mb-1">
                        <span class="text-sm text-gray-700">{"休憩"}</span>
                        <button
                            type="button"
                            class="text-blue-600 text-sm"
                            on:click={
                                let handler = add_break.clone();
                                move |ev| handler(ev)
                            }
                        >
                            {"休憩を追加"}
                        </button>
                    </div>
                    <For
                        each=move || breaks.get()
                        key=|pair| pair.clone()
                        children=move |(s0, e0)| {
                            let s = create_rw_signal(s0);
                            let e = create_rw_signal(e0);
                            view! {
                                <div class="flex space-x-2 mb-2">
                                    <input
                                        type="datetime-local"
                                        class="border rounded px-2 py-1 w-full"
                                        prop:value=s
                                        on:input=move |ev| s.set(event_target_value(&ev))
                                    />
                                    <input
                                        type="datetime-local"
                                        class="border rounded px-2 py-1 w-full"
                                        prop:value=e
                                        on:input=move |ev| e.set(event_target_value(&ev))
                                    />
                                </div>
                            }
                        }
                    />
                </div>
                <button type="submit" class="w-full bg-green-600 text-white rounded py-2">
                    {"保存"}
                </button>
            </form>
            <div class="mt-4">
                <h4 class="text-sm font-medium text-gray-900 mb-2">{"休憩強制終了"}</h4>
                <div class="flex space-x-2">
                    <input
                        placeholder="Break ID"
                        class="border rounded px-2 py-1 w-full"
                        on:input=move |ev| feb_id.set(event_target_value(&ev))
                    />
                    <button
                        class="px-3 py-1 bg-amber-600 text-white rounded"
                        on:click={
                            let handler = on_force_end.clone();
                            move |ev| handler(ev)
                        }
                    >
                        {"強制終了"}
                    </button>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn RequestDetailModal(
    show_modal: RwSignal<bool>,
    modal_data: RwSignal<serde_json::Value>,
    modal_comment: RwSignal<String>,
    on_reject: Rc<dyn Fn(leptos::ev::MouseEvent)>,
    on_approve: Rc<dyn Fn(leptos::ev::MouseEvent)>,
) -> impl IntoView {
    view! {
        <Show when=move || show_modal.get()>
            <div class="fixed inset-0 bg-black/30 flex items-center justify-center z-50">
                <div class="bg-white rounded-lg shadow-lg w-full max-w-lg p-6">
                    <h3 class="text-lg font-medium text-gray-900 mb-2">{"申請詳細"}</h3>
                    <pre class="text-xs bg-gray-50 p-2 rounded overflow-auto max-h-64">
                        {move || format!("{}", modal_data.get())}
                    </pre>
                    <div class="mt-3">
                        <label class="block text-sm font-medium text-gray-700">{"コメント（任意）"}</label>
                        <textarea
                            class="w-full border rounded px-2 py-1"
                            on:input=move |ev| modal_comment.set(event_target_value(&ev))
                        ></textarea>
                    </div>
                    <div class="mt-4 flex justify-end space-x-2">
                        <button class="px-3 py-1 rounded border" on:click=move |_| show_modal.set(false)>
                            {"閉じる"}
                        </button>
                        <button
                            class="px-3 py-1 rounded bg-red-600 text-white"
                            on:click={
                                let handler = on_reject.clone();
                                move |ev| handler(ev)
                            }
                        >
                            {"却下"}
                        </button>
                        <button
                            class="px-3 py-1 rounded bg-green-600 text-white"
                            on:click={
                                let handler = on_approve.clone();
                                move |ev| handler(ev)
                            }
                        >
                            {"承認"}
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}

#[component]
pub fn AdminHolidayListCard(
    admin_holiday_total: RwSignal<i64>,
    admin_holiday_total_pages: Memo<u32>,
    admin_holiday_page: RwSignal<u32>,
    admin_holiday_per_page: RwSignal<u32>,
    admin_holiday_type: RwSignal<String>,
    admin_holiday_from: RwSignal<String>,
    admin_holiday_to: RwSignal<String>,
    admin_holiday_loading: RwSignal<bool>,
    admin_holiday_error: RwSignal<Option<String>>,
    admin_holiday_items: RwSignal<Vec<AdminHolidayListItem>>,
    fetch_admin_holidays: Rc<dyn Fn()>,
) -> impl IntoView {
    view! {
        <div class="bg-white shadow rounded-lg p-6 space-y-4">
            <div class="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
                <div>
                    <h2 class="text-lg font-semibold text-gray-900">{"休日一覧（ページネーション）"}</h2>
                    <p class="text-sm text-gray-600">
                        {"種類と期間、対象の登録状況をまとめて確認できます。フィルタを設定して再取得を実行してください。"}
                    </p>
                </div>
                <div class="flex items-center gap-2 text-sm text-gray-700">
                    <span>
                        {move || {
                            let total = admin_holiday_total.get();
                            let current = admin_holiday_page.get();
                            let max = admin_holiday_total_pages.get();
                            format!("全{}件 / {}/{}ページ", total, current, max)
                        }}
                    </span>
                    <div class="flex gap-1">
                        <button
                            class="px-3 py-1 rounded border text-sm hover:bg-gray-50 disabled:opacity-50"
                            on:click={
                                let page = admin_holiday_page.clone();
                                let fetch = fetch_admin_holidays.clone();
                                move |_ev: leptos::ev::MouseEvent| {
                                    if page.get() > 1 {
                                        page.update(|p| *p -= 1);
                                        fetch();
                                    }
                                }
                            }
                            disabled=move || admin_holiday_page.get() <= 1 || admin_holiday_loading.get()
                        >
                            {"前へ"}
                        </button>
                        <button
                            class="px-3 py-1 rounded border text-sm hover:bg-gray-50 disabled:opacity-50"
                            on:click={
                                let page = admin_holiday_page.clone();
                                let fetch = fetch_admin_holidays.clone();
                                move |_ev: leptos::ev::MouseEvent| {
                                    let max_page = admin_holiday_total_pages.get();
                                    if page.get() < max_page {
                                        page.update(|p| *p += 1);
                                        fetch();
                                    }
                                }
                            }
                            disabled=move || {
                                admin_holiday_page.get() >= admin_holiday_total_pages.get()
                                    || admin_holiday_loading.get()
                            }
                        >
                            {"次へ"}
                        </button>
                    </div>
                </div>
            </div>
            <div class="grid gap-3 md:grid-cols-5">
                <div>
                    <label class="block text-sm font-medium text-gray-700">{"種別"}</label>
                    <select
                        class="mt-1 w-full border rounded px-2 py-1"
                        prop:value={move || admin_holiday_type.get()}
                        on:change=move |ev| admin_holiday_type.set(event_target_value(&ev))
                    >
                        <option value="all">{"すべて"}</option>
                        <option value="public">{"祝日"}</option>
                        <option value="weekly">{"定休"}</option>
                        <option value="exception">{"例外"}</option>
                    </select>
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700">{"適用開始 (from)"}</label>
                    <input
                        class="mt-1 w-full border rounded px-2 py-1"
                        type="date"
                        prop:value={move || admin_holiday_from.get()}
                        on:input=move |ev| admin_holiday_from.set(event_target_value(&ev))
                    />
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700">{"適用終了 (to)"}</label>
                    <input
                        class="mt-1 w-full border rounded px-2 py-1"
                        type="date"
                        prop:value={move || admin_holiday_to.get()}
                        on:input=move |ev| admin_holiday_to.set(event_target_value(&ev))
                    />
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700">{"1ページの件数"}</label>
                    <select
                        class="mt-1 w-full border rounded px-2 py-1"
                        prop:value={move || admin_holiday_per_page.get().to_string()}
                        on:change={
                            let per_page_signal = admin_holiday_per_page.clone();
                            let page_signal = admin_holiday_page.clone();
                            let fetch = fetch_admin_holidays.clone();
                            move |ev| {
                                let raw = event_target_value(&ev);
                                let parsed = raw
                                    .parse::<u32>()
                                    .ok()
                                    .filter(|value| *value > 0)
                                    .unwrap_or(25);
                                if per_page_signal.get_untracked() != parsed {
                                    per_page_signal.set(parsed);
                                    page_signal.set(1);
                                    fetch();
                                }
                            }
                        }
                    >
                        <option value="10">{"10件"}</option>
                        <option value="25">{"25件"}</option>
                        <option value="50">{"50件"}</option>
                        <option value="100">{"100件"}</option>
                    </select>
                </div>
                <div class="flex items-end">
                    <button
                        class="w-full px-3 py-2 rounded bg-blue-600 text-white hover:bg-blue-700 disabled:opacity-50"
                        on:click={
                            let page = admin_holiday_page.clone();
                            let fetch = fetch_admin_holidays.clone();
                            move |_ev: leptos::ev::MouseEvent| {
                                page.set(1);
                                fetch();
                            }
                        }
                        disabled=move || admin_holiday_loading.get()
                    >
                        {"フィルタ適用"}
                    </button>
                </div>
            </div>
            <Show when=move || admin_holiday_error.get().is_some()>
                {move || {
                    view! {
                        <ErrorMessage
                            message=admin_holiday_error
                                .get()
                                .unwrap_or_else(|| "休日データの取得に失敗しました。".into())
                        />
                    }
                }}
            </Show>
            <Show when=move || admin_holiday_loading.get()>
                <div class="text-sm text-gray-600">{"読み込み中..."}</div>
            </Show>
            <Show when=move || !admin_holiday_loading.get() && admin_holiday_items.get().is_empty()>
                <p class="text-sm text-gray-600">{"対象の休日日程はありません。フィルタ条件を変更してみてください。"}</p>
            </Show>
            <Show when=move || !admin_holiday_items.get().is_empty()>
                <div class="overflow-x-auto">
                    <table class="min-w-full divide-y divide-gray-200">
                        <thead class="bg-gray-50">
                            <tr>
                                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                                    {"適用日"}
                                </th>
                                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                                    {"種別"}
                                </th>
                                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                                    {"概要"}
                                </th>
                                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                                    {"詳細"}
                                </th>
                            </tr>
                        </thead>
                        <tbody class="bg-white divide-y divide-gray-200">
                            <For
                                each=move || admin_holiday_items.get()
                                key=|item| item.id.clone()
                                children=move |item| {
                                    let range = admin_holiday_range(&item);
                                    let label = describe_admin_holiday(&item);
                                    let detail = item
                                        .reason
                                        .clone()
                                        .or(item.description.clone())
                                        .unwrap_or_else(|| "-".into());
                                    view! {
                                        <tr>
                                            <td class="px-4 py-2 text-sm text-gray-900">{range}</td>
                                            <td class="px-4 py-2 text-sm capitalize text-gray-900">{item.kind.clone()}</td>
                                            <td class="px-4 py-2 text-sm text-gray-900">{label}</td>
                                            <td class="px-4 py-2 text-sm text-gray-600">{detail}</td>
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
fn delete_holiday_handler(
    holiday_id: String,
    holiday_label: String,
    holidays_signal: RwSignal<Vec<HolidayResponse>>,
    holiday_message_signal: RwSignal<Option<String>>,
    holiday_error_signal: RwSignal<Option<String>>,
    holiday_deleting_signal: RwSignal<Option<String>>,
) -> impl Fn(leptos::ev::MouseEvent) {
    move |_| {
        holiday_error_signal.set(None);
        holiday_message_signal.set(None);
        holiday_deleting_signal.set(Some(holiday_id.clone()));
        let holidays_signal = holidays_signal.clone();
        let holiday_message_signal = holiday_message_signal.clone();
        let holiday_error_signal = holiday_error_signal.clone();
        let holiday_deleting_signal = holiday_deleting_signal.clone();
        let id_for_task = holiday_id.clone();
        let label_for_task = holiday_label.clone();
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
}
