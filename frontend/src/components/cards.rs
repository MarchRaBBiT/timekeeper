use chrono::{Datelike, Timelike};
use leptos::*;

use crate::state::attendance::AttendanceState;
use crate::utils::time::today_in_app_tz;

#[component]
pub fn AttendanceCard(attendance_state: ReadSignal<AttendanceState>) -> impl IntoView {
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
        <div class="bg-surface-elevated overflow-hidden shadow rounded-lg">
            <div class="px-4 py-5 sm:p-6">
                <h3 class="text-lg leading-6 font-medium text-fg">
                    {format!("今日の勤怠 ({:04}-{:02}-{:02})", today.year(), today.month(), today.day())}
                </h3>
                <div class="mt-5">
                    <div class="grid grid-cols-2 gap-4">
                        <div>
                            <dt class="text-sm font-medium text-fg-muted">{"出勤時間"}</dt>
                            <dd class="mt-1 text-sm text-fg">{clock_in}</dd>
                        </div>
                        <div>
                            <dt class="text-sm font-medium text-fg-muted">{"退勤時間"}</dt>
                            <dd class="mt-1 text-sm text-fg">{clock_out}</dd>
                        </div>
                        <div>
                            <dt class="text-sm font-medium text-fg-muted">{"稼働時間"}</dt>
                            <dd class="mt-1 text-sm text-fg">{total_hours}</dd>
                        </div>
                        <div>
                            <dt class="text-sm font-medium text-fg-muted">{"休憩合計"}</dt>
                            <dd class="mt-1 text-sm text-fg">{break_minutes}</dd>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn SummaryCard(
    summary: Resource<(), Result<crate::pages::dashboard::repository::DashboardSummary, String>>,
) -> impl IntoView {
    let summary_val = move || summary.get().and_then(|res| res.ok());

    view! {
        <div class="bg-surface-elevated overflow-hidden shadow rounded-lg">
            <div class="px-4 py-5 sm:p-6">
                <h3 class="text-lg leading-6 font-medium text-fg">{"今月のサマリー"}</h3>
                <div class="mt-5">
                    <div class="grid grid-cols-3 gap-4">
                        <div>
                            <dt class="text-sm font-medium text-fg-muted">{"総稼働時間"}</dt>
                            <dd class="mt-1 text-2xl font-semibold text-fg">
                                {move || summary_val().and_then(|s| s.total_work_hours).map(|h| format!("{:.2}時間", h)).unwrap_or_else(|| "-".into())}
                            </dd>
                        </div>
                        <div>
                            <dt class="text-sm font-medium text-fg-muted">{"稼働日数"}</dt>
                            <dd class="mt-1 text-2xl font-semibold text-fg">
                                {move || summary_val().and_then(|s| s.total_work_days).map(|d| format!("{} 日", d)).unwrap_or_else(|| "-".into())}
                            </dd>
                        </div>
                        <div>
                            <dt class="text-sm font-medium text-fg-muted">{"平均稼働時間"}</dt>
                            <dd class="mt-1 text-2xl font-semibold text-fg">
                                {move || summary_val().and_then(|s| s.average_daily_hours).map(|h| format!("{:.2}時間", h)).unwrap_or_else(|| "-".into())}
                            </dd>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn RequestCard(
    requests: Resource<
        crate::pages::dashboard::utils::ActivityStatusFilter,
        Result<Vec<crate::pages::dashboard::repository::DashboardActivity>, String>,
    >,
) -> impl IntoView {
    let activities = move || requests.get().and_then(|res| res.ok()).unwrap_or_default();

    view! {
        <div class="bg-surface-elevated overflow-hidden shadow rounded-lg">
            <div class="px-4 py-5 sm:p-6">
                <h3 class="text-lg leading-6 font-medium text-fg">{"申請状況"}</h3>
                <div class="mt-5 space-y-2">
                    <For
                        each=activities
                        key=|a| a.title.clone()
                        children=|a| {
                            view! {
                                <div class="text-sm text-fg">
                                    {a.title} {": "} {a.detail.unwrap_or_else(|| "-".into())}
                                </div>
                            }
                        }
                    />
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
        <div class="bg-surface-elevated overflow-hidden shadow rounded-lg">
            <div class="px-4 py-5 sm:p-6">
                <h3 class="text-lg leading-6 font-medium text-fg">{"ユーザー情報"}</h3>
                <div class="mt-5">
                    <div class="space-y-2">
                        <div>
                            <dt class="text-sm font-medium text-fg-muted">{"ユーザー名"}</dt>
                            <dd class="mt-1 text-sm text-fg">{move || auth.get().user.as_ref().map(|u| u.username.clone()).unwrap_or_else(|| "-".into())}</dd>
                        </div>
                        <div>
                            <dt class="text-sm font-medium text-fg-muted">{"氏名"}</dt>
                            <dd class="mt-1 text-sm text-fg">{move || auth.get().user.as_ref().map(|u| u.full_name.clone()).unwrap_or_else(|| "-".into())}</dd>
                        </div>
                        <div>
                            <dt class="text-sm font-medium text-fg-muted">{"役割"}</dt>
                            <dd class="mt-1 text-sm text-fg">{move || auth.get().user.as_ref().map(|u| u.role.clone()).unwrap_or_else(|| "-".into())}</dd>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::api::{AttendanceResponse, AttendanceStatusResponse, BreakRecordResponse};
    use crate::pages::dashboard::repository::{DashboardActivity, DashboardSummary};
    use crate::pages::dashboard::utils::ActivityStatusFilter;
    use crate::test_support::helpers::{admin_user, provide_auth};
    use crate::test_support::ssr::render_to_string;

    fn today_datetime(hour: u32, minute: u32) -> chrono::NaiveDateTime {
        let today = crate::utils::time::today_in_app_tz();
        today.and_hms_opt(hour, minute, 0).unwrap()
    }

    #[test]
    fn attendance_card_renders_with_values() {
        let today = crate::utils::time::today_in_app_tz();
        let attendance = AttendanceResponse {
            id: "att-1".into(),
            user_id: "u1".into(),
            date: today,
            clock_in_time: Some(today_datetime(9, 0)),
            clock_out_time: Some(today_datetime(18, 0)),
            status: "clocked_in".into(),
            total_work_hours: Some(8.0),
            break_records: vec![BreakRecordResponse {
                id: "br-1".into(),
                attendance_id: "att-1".into(),
                break_start_time: today_datetime(12, 0),
                break_end_time: Some(today_datetime(12, 30)),
                duration_minutes: Some(30),
            }],
        };
        let status = AttendanceStatusResponse {
            status: "clocked_in".into(),
            attendance_id: Some("att-1".into()),
            active_break_id: None,
            clock_in_time: Some(today_datetime(9, 0)),
            clock_out_time: Some(today_datetime(18, 0)),
        };
        let state = AttendanceState {
            current_attendance: Some(attendance.clone()),
            attendance_history: vec![attendance],
            today_status: Some(status),
            today_holiday_reason: None,
            last_refresh_error: None,
            range_from: Some(today),
            range_to: Some(today),
            loading: false,
        };

        let html = render_to_string(move || {
            let (signal, _) = create_signal(state);
            view! { <AttendanceCard attendance_state=signal /> }
        });
        assert!(html.contains("今日の勤怠"));
        assert!(html.contains("30"));
    }

    #[test]
    fn summary_card_renders_values() {
        let html = render_to_string(move || {
            let summary = Resource::new(
                || (),
                |_| async move {
                    Ok(DashboardSummary {
                        total_work_hours: Some(160.0),
                        total_work_days: Some(20),
                        average_daily_hours: Some(8.0),
                    })
                },
            );
            summary.set(Ok(DashboardSummary {
                total_work_hours: Some(160.0),
                total_work_days: Some(20),
                average_daily_hours: Some(8.0),
            }));
            view! { <SummaryCard summary=summary /> }
        });
        assert!(html.contains("今月のサマリー"));
    }

    #[test]
    fn request_card_renders_activity_list() {
        let html = render_to_string(move || {
            let activities = Resource::new(
                || ActivityStatusFilter::All,
                |_| async move {
                    Ok(vec![DashboardActivity {
                        title: "休暇申請（承認待ち）".into(),
                        detail: Some("1 件".into()),
                    }])
                },
            );
            activities.set(Ok(vec![DashboardActivity {
                title: "休暇申請（承認待ち）".into(),
                detail: Some("1 件".into()),
            }]));
            view! { <RequestCard requests=activities /> }
        });
        assert!(html.contains("申請状況"));
    }

    #[test]
    fn user_card_renders_user_info() {
        let html = render_to_string(move || {
            provide_auth(Some(admin_user(true)));
            view! { <UserCard /> }
        });
        assert!(html.contains("ユーザー情報"));
        assert!(html.contains("admin"));
    }
}
