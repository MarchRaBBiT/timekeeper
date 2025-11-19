use crate::state::attendance::{
    self as attendance_state, describe_holiday_reason, AttendanceState,
};
use leptos::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct AttendanceButtonFlags {
    clock_in: bool,
    break_start: bool,
    break_end: bool,
    clock_out: bool,
}

fn button_flags_for(status: Option<&str>, loading: bool) -> AttendanceButtonFlags {
    if loading {
        return AttendanceButtonFlags::default();
    }

    match status.unwrap_or("not_started") {
        "not_started" => AttendanceButtonFlags {
            clock_in: true,
            ..Default::default()
        },
        "clocked_in" => AttendanceButtonFlags {
            break_start: true,
            clock_out: true,
            ..Default::default()
        },
        "on_break" => AttendanceButtonFlags {
            break_end: true,
            clock_out: true,
            ..Default::default()
        },
        "clocked_out" => AttendanceButtonFlags::default(),
        _ => AttendanceButtonFlags::default(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ClockEventKind {
    ClockIn,
    BreakStart,
    BreakEnd,
    ClockOut,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ClockEventPayload {
    kind: ClockEventKind,
    attendance_id: Option<String>,
    break_id: Option<String>,
}

impl ClockEventPayload {
    fn clock_in() -> Self {
        Self {
            kind: ClockEventKind::ClockIn,
            attendance_id: None,
            break_id: None,
        }
    }

    fn clock_out() -> Self {
        Self {
            kind: ClockEventKind::ClockOut,
            attendance_id: None,
            break_id: None,
        }
    }

    fn break_start(attendance_id: String) -> Self {
        Self {
            kind: ClockEventKind::BreakStart,
            attendance_id: Some(attendance_id),
            break_id: None,
        }
    }

    fn break_end(break_id: String) -> Self {
        Self {
            kind: ClockEventKind::BreakEnd,
            attendance_id: None,
            break_id: Some(break_id),
        }
    }
}

#[component]
pub fn AttendanceActionButtons(
    attendance_state: ReadSignal<AttendanceState>,
    set_attendance_state: WriteSignal<AttendanceState>,
) -> impl IntoView {
    let (message, set_message) = create_signal(None::<String>);
    let last_event = create_rw_signal(None::<ClockEventKind>);

    let clock_action = {
        let set_attendance_state = set_attendance_state.clone();
        create_action(move |payload: &ClockEventPayload| {
            let set_attendance_state = set_attendance_state.clone();
            let payload = payload.clone();
            async move {
                match payload.kind {
                    ClockEventKind::ClockIn => {
                        attendance_state::clock_in(set_attendance_state.clone()).await?
                    }
                    ClockEventKind::ClockOut => {
                        attendance_state::clock_out(set_attendance_state.clone()).await?
                    }
                    ClockEventKind::BreakStart => {
                        let attendance_id = payload
                            .attendance_id
                            .as_deref()
                            .ok_or_else(|| "出勤レコードが見つかりません。".to_string())?;
                        attendance_state::start_break(attendance_id).await?
                    }
                    ClockEventKind::BreakEnd => {
                        let break_id = payload
                            .break_id
                            .as_deref()
                            .ok_or_else(|| "休憩レコードが見つかりません。".to_string())?;
                        attendance_state::end_break(break_id).await?
                    }
                };
                attendance_state::refresh_today_context(set_attendance_state).await
            }
        })
    };
    let action_pending = clock_action.pending();
    {
        let clock_action = clock_action.clone();
        let set_message = set_message.clone();
        let last_event = last_event.clone();
        create_effect(move |_| {
            if let Some(result) = clock_action.value().get() {
                match result {
                    Ok(_) => {
                        let success = match last_event.get_untracked() {
                            Some(ClockEventKind::ClockIn) => "出勤しました。",
                            Some(ClockEventKind::BreakStart) => "休憩を開始しました。",
                            Some(ClockEventKind::BreakEnd) => "休憩を終了しました。",
                            Some(ClockEventKind::ClockOut) => "退勤しました。",
                            None => "操作が完了しました。",
                        };
                        set_message.set(Some(success.into()));
                    }
                    Err(err) => set_message.set(Some(err)),
                }
            }
        });
    }

    let handle_clock_in = {
        let clock_action = clock_action.clone();
        let action_pending = action_pending;
        let set_message = set_message.clone();
        let last_event = last_event.clone();
        move |_| {
            if action_pending.get() {
                return;
            }
            set_message.set(None);
            last_event.set(Some(ClockEventKind::ClockIn));
            clock_action.dispatch(ClockEventPayload::clock_in());
        }
    };

    let handle_clock_out = {
        let clock_action = clock_action.clone();
        let action_pending = action_pending;
        let set_message = set_message.clone();
        let last_event = last_event.clone();
        move |_| {
            if action_pending.get() {
                return;
            }
            set_message.set(None);
            last_event.set(Some(ClockEventKind::ClockOut));
            clock_action.dispatch(ClockEventPayload::clock_out());
        }
    };

    let handle_break_start = {
        let attendance_state = attendance_state.clone();
        let set_message = set_message.clone();
        let clock_action = clock_action.clone();
        let action_pending = action_pending;
        let last_event = last_event.clone();
        move |_| {
            if action_pending.get() {
                return;
            }
            let Some(status) = attendance_state.get().today_status.clone() else {
                set_message.set(Some("ステータスを取得できません。".into()));
                return;
            };
            if status.status != "clocked_in" {
                set_message.set(Some("出勤中のみ休憩を開始できます。".into()));
                return;
            }
            let Some(att_id) = status.attendance_id.clone() else {
                set_message.set(Some("出勤レコードが見つかりません。".into()));
                return;
            };
            set_message.set(None);
            last_event.set(Some(ClockEventKind::BreakStart));
            clock_action.dispatch(ClockEventPayload::break_start(att_id));
        }
    };

    let handle_break_end = {
        let attendance_state = attendance_state.clone();
        let set_message = set_message.clone();
        let clock_action = clock_action.clone();
        let action_pending = action_pending;
        let last_event = last_event.clone();
        move |_| {
            if action_pending.get() {
                return;
            }
            let Some(status) = attendance_state.get().today_status.clone() else {
                set_message.set(Some("ステータスを取得できません。".into()));
                return;
            };
            if status.status != "on_break" {
                set_message.set(Some("休憩中のみ休憩を終了できます。".into()));
                return;
            }
            let Some(break_id) = status.active_break_id.clone() else {
                set_message.set(Some("休憩レコードが見つかりません。".into()));
                return;
            };
            set_message.set(None);
            last_event.set(Some(ClockEventKind::BreakEnd));
            clock_action.dispatch(ClockEventPayload::break_end(break_id));
        }
    };

    let status_snapshot = move || attendance_state.get().today_status.clone();
    let holiday_reason = create_memo(move |_| attendance_state.get().today_holiday_reason.clone());
    let button_state = move || {
        let flags = button_flags_for(
            status_snapshot().as_ref().map(|s| s.status.as_str()),
            action_pending.get(),
        );
        if holiday_reason.get().is_some() {
            AttendanceButtonFlags::default()
        } else {
            flags
        }
    };

    view! {
        <div class="space-y-3">
            <div class="flex flex-wrap items-center gap-4">
                {move || {
                    let status = status_snapshot();
                    let (label, color) = match status.as_ref().map(|s| s.status.as_str()) {
                        Some("clocked_in") => ("勤務中", "bg-blue-100 text-blue-800"),
                        Some("on_break") => ("休憩中", "bg-yellow-100 text-yellow-800"),
                        Some("clocked_out") => ("退勤済み", "bg-green-100 text-green-800"),
                        _ => ("未出勤", "bg-gray-100 text-gray-800"),
                    };
                    view! {
                        <span class=format!(
                            "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {}",
                            color
                        )>
                            {label}
                        </span>
                    }
                    .into_view()
                }}
                <div class="flex flex-wrap gap-3">
                    <button
                        class="px-4 py-2 rounded bg-indigo-600 text-white disabled:opacity-50"
                        disabled={move || !button_state().clock_in}
                        on:click=handle_clock_in
                    >
                        {"出勤"}
                    </button>
                    <button
                        class="px-4 py-2 rounded bg-amber-600 text-white disabled:opacity-50"
                        disabled={move || !button_state().break_start}
                        on:click=handle_break_start
                    >
                        {"休憩開始"}
                    </button>
                    <button
                        class="px-4 py-2 rounded bg-amber-700 text-white disabled:opacity-50"
                        disabled={move || !button_state().break_end}
                        on:click=handle_break_end
                    >
                        {"休憩終了"}
                    </button>
                    <button
                        class="px-4 py-2 rounded bg-red-600 text-white disabled:opacity-50"
                        disabled={move || !button_state().clock_out}
                        on:click=handle_clock_out
                    >
                        {"退勤"}
                    </button>
                </div>
            </div>
            {move || {
                holiday_reason
                    .get()
                    .map(|reason| {
                        let label = describe_holiday_reason(reason.trim());
                        view! {
                            <div class="bg-amber-50 border border-amber-200 text-amber-800 px-3 py-2 rounded text-sm">
                                {format!("本日は{}のため打刻できません。", label)}
                            </div>
                        }
                        .into_view()
                    })
                    .unwrap_or_else(|| view! {}.into_view())
            }}
            <Show when=move || action_pending.get()>
                <p class="text-sm text-gray-500">{"処理中です..."}</p>
            </Show>
            {move || {
                message
                    .get()
                    .map(|msg| view! { <p class="text-sm text-gray-700">{msg}</p> }.into_view())
                    .unwrap_or_else(|| view! {}.into_view())
            }}
        </div>
    }
}
