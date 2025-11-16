use chrono::{Datelike, Timelike};
use leptos::*;

use crate::api::{ApiClient, AttendanceSummary};
use crate::state::attendance::{refresh_today_context, use_attendance};
use crate::utils::time::{now_in_app_tz, today_in_app_tz};

#[component]
pub fn AttendanceCard() -> impl IntoView {
    let (attendance_state, set_attendance_state) = use_attendance();

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
                    {format!("本日の勤怠 ({:04}-{:02}-{:02})", today.year(), today.month(), today.day())}
                </h3>
                <div class="mt-5">
                    <div class="grid grid-cols-2 gap-4">
                        <div>
                            <dt class="text-sm font-medium text-gray-500">{"出勤時刻"}</dt>
                            <dd class="mt-1 text-sm text-gray-900">{clock_in}</dd>
                        </div>
                        <div>
                            <dt class="text-sm font-medium text-gray-500">{"退勤時刻"}</dt>
                            <dd class="mt-1 text-sm text-gray-900">{clock_out}</dd>
                        </div>
                        <div>
                            <dt class="text-sm font-medium text-gray-500">{"総労働時間"}</dt>
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
                <h3 class="text-lg leading-6 font-medium text-gray-900">{"月次サマリ"}</h3>
                <div class="mt-5">
                    <div class="grid grid-cols-3 gap-4">
                        <div>
                            <dt class="text-sm font-medium text-gray-500">{"総労働時間"}</dt>
                            <dd class="mt-1 text-2xl font-semibold text-gray-900">
                                {move || summary.get().as_ref().map(|s| format!("{:.2} 時間", s.total_work_hours)).unwrap_or_else(|| "-".into())}
                            </dd>
                        </div>
                        <div>
                            <dt class="text-sm font-medium text-gray-500">{"労働日数"}</dt>
                            <dd class="mt-1 text-2xl font-semibold text-gray-900">
                                {move || summary.get().as_ref().map(|s| format!("{} 日", s.total_work_days)).unwrap_or_else(|| "-".into())}
                            </dd>
                        </div>
                        <div>
                            <dt class="text-sm font-medium text-gray-500">{"平均労働時間"}</dt>
                            <dd class="mt-1 text-2xl font-semibold text-gray-900">
                                {move || summary.get().as_ref().map(|s| format!("{:.2} 時間", s.average_daily_hours)).unwrap_or_else(|| "-".into())}
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
                    <div class="text-sm text-gray-700">{"休暇申請: 待ち="}{move || count("leave_requests","pending")} {" 件 / 承認="}{move || count("leave_requests","approved")} {" 件 / 却下="}{move || count("leave_requests","rejected")} {" 件"}</div>
                    <div class="text-sm text-gray-700">{"残業申請: 待ち="}{move || count("overtime_requests","pending")} {" 件 / 承認="}{move || count("overtime_requests","approved")} {" 件 / 却下="}{move || count("overtime_requests","rejected")} {" 件"}</div>
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
                            <dt class="text-sm font-medium text-gray-500">{"権限"}</dt>
                            <dd class="mt-1 text-sm text-gray-900">{move || auth.get().user.as_ref().map(|u| u.role.clone()).unwrap_or_else(|| "-".into())}</dd>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}
