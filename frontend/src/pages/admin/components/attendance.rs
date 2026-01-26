use crate::{
    api::{AdminAttendanceUpsert, AdminBreakItem, ApiError},
    components::{error::InlineErrorMessage, forms::DatePicker, layout::SuccessMessage},
    pages::admin::{
        components::user_select::{AdminUserSelect, UsersResource},
        repository::AdminRepository,
    },
};
use chrono::{NaiveDate, NaiveDateTime};
use leptos::{ev, *};

fn parse_dt_local(input: &str) -> Option<NaiveDateTime> {
    if input.len() == 16 {
        NaiveDateTime::parse_from_str(&format!("{}:00", input), "%Y-%m-%dT%H:%M:%S").ok()
    } else {
        NaiveDateTime::parse_from_str(input, "%Y-%m-%dT%H:%M:%S").ok()
    }
}

#[component]
pub fn AdminAttendanceToolsSection(
    repository: AdminRepository,
    system_admin_allowed: Memo<bool>,
    users: UsersResource,
) -> impl IntoView {
    let att_user = create_rw_signal(String::new());
    let att_date = create_rw_signal(String::new());
    let att_in = create_rw_signal(String::new());
    let att_out = create_rw_signal(String::new());
    let breaks = create_rw_signal(Vec::<(String, String)>::new());
    let break_force_id = create_rw_signal(String::new());
    let message = create_rw_signal(None::<String>);
    let error = create_rw_signal(None::<ApiError>);

    let add_break = {
        move |_| {
            breaks.update(|list| list.push((String::new(), String::new())));
        }
    };

    let repo_for_attendance = repository.clone();
    let attendance_action = create_action(move |payload: &AdminAttendanceUpsert| {
        let repo = repo_for_attendance.clone();
        let payload = payload.clone();
        async move { repo.upsert_attendance(payload).await }
    });
    let attendance_pending = attendance_action.pending();

    let repo_for_break = repository.clone();
    let force_break_action = create_action(move |break_id: &String| {
        let repo = repo_for_break.clone();
        let break_id = break_id.clone();
        async move { repo.force_end_break(&break_id).await }
    });
    let force_pending = force_break_action.pending();

    {
        create_effect(move |_| {
            if let Some(result) = attendance_action.value().get() {
                match result {
                    Ok(_) => {
                        message.set(Some("勤怠データを登録しました。".into()));
                        error.set(None);
                    }
                    Err(err) => {
                        message.set(None);
                        error.set(Some(err));
                    }
                }
            }
        });
    }
    {
        create_effect(move |_| {
            if let Some(result) = force_break_action.value().get() {
                match result {
                    Ok(_) => {
                        message.set(Some("休憩を強制終了しました。".into()));
                        error.set(None);
                    }
                    Err(err) => {
                        message.set(None);
                        error.set(Some(err));
                    }
                }
            }
        });
    }

    let on_submit_attendance = {
        move |ev: ev::SubmitEvent| {
            ev.prevent_default();
            if !system_admin_allowed.get_untracked() {
                return;
            }
            let user_id = att_user.get();
            let date_raw = att_date.get();
            let clock_in = parse_dt_local(&att_in.get());
            if user_id.trim().is_empty() || date_raw.trim().is_empty() || clock_in.is_none() {
                error.set(Some(ApiError::validation(
                    "ユーザーID・日付・出勤時刻を入力してください。",
                )));
                message.set(None);
                return;
            }
            let clock_out = if att_out.get().trim().is_empty() {
                None
            } else {
                parse_dt_local(&att_out.get())
            };
            let date = NaiveDate::parse_from_str(&date_raw, "%Y-%m-%d").ok();
            if date.is_none() {
                error.set(Some(ApiError::validation(
                    "日付は YYYY-MM-DD 形式で入力してください。",
                )));
                message.set(None);
                return;
            }
            let mut break_items: Vec<AdminBreakItem> = vec![];
            for (start, end) in breaks.get() {
                if start.trim().is_empty() {
                    continue;
                }
                let start_dt = parse_dt_local(&start);
                let end_dt = if end.trim().is_empty() {
                    None
                } else {
                    parse_dt_local(&end)
                };
                if let Some(start_dt) = start_dt {
                    break_items.push(AdminBreakItem {
                        break_start_time: start_dt,
                        break_end_time: end_dt,
                    });
                }
            }
            let payload = AdminAttendanceUpsert {
                user_id,
                date: date.unwrap(),
                clock_in_time: clock_in.unwrap(),
                clock_out_time: clock_out,
                breaks: if break_items.is_empty() {
                    None
                } else {
                    Some(break_items)
                },
            };
            message.set(None);
            error.set(None);
            attendance_action.dispatch(payload);
        }
    };

    let on_force_end = {
        move |_| {
            if !system_admin_allowed.get_untracked() {
                return;
            }
            let id = break_force_id.get();
            if id.trim().is_empty() {
                error.set(Some(ApiError::validation("Break ID を入力してください。")));
                message.set(None);
                return;
            }
            error.set(None);
            message.set(None);
            force_break_action.dispatch(id);
        }
    };

    view! {
        <Show when=move || system_admin_allowed.get()>
            <div class="bg-surface-elevated shadow rounded-lg p-6 space-y-4">
                <h3 class="text-lg font-medium text-fg">{"勤怠ツール"}</h3>
                <form class="space-y-3" on:submit=on_submit_attendance>
                    <AdminUserSelect
                        users=users
                        selected=att_user
                        label=Some("対象ユーザー".into())
                        placeholder="ユーザーを選択してください".into()
                    />
                    <DatePicker
                        label=Some("対象日")
                        value=att_date
                    />
                    <input type="datetime-local" class="w-full border border-form-control-border bg-form-control-bg text-form-control-text rounded px-2 py-1" on:input=move |ev| att_in.set(event_target_value(&ev)) />
                    <input type="datetime-local" class="w-full border border-form-control-border bg-form-control-bg text-form-control-text rounded px-2 py-1" on:input=move |ev| att_out.set(event_target_value(&ev)) />
                    <div>
                        <div class="flex items-center justify-between mb-1">
                            <span class="text-sm text-fg-muted">{"休憩（任意）"}</span>
                            <button type="button" class="text-link hover:text-link-hover text-sm" on:click=add_break>{"行を追加"}</button>
                        </div>
                        <For
                            each=move || breaks.get()
                            key=|pair| pair.clone()
                            children=move |(s0, e0)| {
                                let s = create_rw_signal(s0);
                                let e = create_rw_signal(e0);
                                view! {
                                    <div class="flex space-x-2 mb-2">
                                        <input type="datetime-local" class="border border-form-control-border bg-form-control-bg text-form-control-text rounded px-2 py-1 w-full" prop:value=s on:input=move |ev| s.set(event_target_value(&ev)) />
                                        <input type="datetime-local" class="border border-form-control-border bg-form-control-bg text-form-control-text rounded px-2 py-1 w-full" prop:value=e on:input=move |ev| e.set(event_target_value(&ev)) />
                                    </div>
                                }
                            }
                        />
                    </div>
                    <button
                        type="submit"
                        class="w-full bg-action-primary-bg text-action-primary-text rounded py-2 disabled:opacity-50"
                        disabled={move || attendance_pending.get()}
                    >
                        {move || if attendance_pending.get() { "登録中..." } else { "勤怠を登録" }}
                    </button>
                </form>
                <div class="mt-4">
                    <h4 class="text-sm font-medium text-fg mb-2">{"休憩の強制終了"}</h4>
                    <div class="flex space-x-2">
                        <input
                            placeholder="Break ID"
                            class="border border-form-control-border bg-form-control-bg text-form-control-text rounded px-2 py-1 w-full"
                            on:input=move |ev| break_force_id.set(event_target_value(&ev))
                        />
                        <button
                            class="px-3 py-1 bg-action-danger-bg text-action-danger-text rounded disabled:opacity-50"
                            disabled={move || force_pending.get()}
                            on:click=on_force_end
                        >
                            {move || if force_pending.get() { "終了中..." } else { "強制終了" }}
                        </button>
                    </div>
                </div>
                <Show when=move || error.get().is_some()>
                <InlineErrorMessage error={error.into()} />
            </Show>
                <Show when=move || message.get().is_some()>
                    <SuccessMessage message={message.get().unwrap_or_default()} />
                </Show>
            </div>
        </Show>
    }
}
