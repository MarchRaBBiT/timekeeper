use crate::api::ApiClient;
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

#[component]
pub fn AttendanceActionButtons(
    attendance_state: ReadSignal<AttendanceState>,
    set_attendance_state: WriteSignal<AttendanceState>,
) -> impl IntoView {
    let (loading, set_loading) = create_signal(false);
    let (message, set_message) = create_signal(None::<String>);

    let refresh_state = {
        let set_attendance_state = set_attendance_state.clone();
        move || {
            let set_state = set_attendance_state.clone();
            async move { attendance_state::refresh_today_context(set_state).await }
        }
    };

    let handle_clock_in = {
        let set_attendance_state = set_attendance_state.clone();
        let set_message = set_message.clone();
        let set_loading = set_loading.clone();
        move |_| {
            if loading.get() {
                return;
            }
            set_loading.set(true);
            set_message.set(None);
            let set_state = set_attendance_state.clone();
            let set_message = set_message.clone();
            let set_loading = set_loading.clone();
            spawn_local(async move {
                match attendance_state::clock_in(set_state.clone()).await {
                    Ok(_) => match refresh_state().await {
                        Ok(_) => set_message.set(Some("打刻しました。".into())),
                        Err(err) => {
                            log::error!("Failed to refresh attendance context: {}", err);
                            set_message.set(Some(format!("状態の更新に失敗しました: {}", err)));
                        }
                    },
                    Err(err) => set_message.set(Some(format!("打刻に失敗しました: {}", err))),
                };
                set_loading.set(false);
            });
        }
    };

    let handle_break_start = {
        let attendance_state = attendance_state.clone();
        let set_message = set_message.clone();
        move |_| {
            if loading.get() {
                return;
            }
            let Some(status) = attendance_state.get().today_status.clone() else {
                set_message.set(Some("ステータスを取得できません。".into()));
                return;
            };
            if status.status != "clocked_in" {
                set_message.set(Some("勤務中のみ休憩を開始できます。".into()));
                return;
            }
            let Some(att_id) = status.attendance_id.clone() else {
                set_message.set(Some("勤怠レコードが見つかりません。".into()));
                return;
            };
            set_loading.set(true);
            set_message.set(None);
            let set_message = set_message.clone();
            let set_loading = set_loading.clone();
            spawn_local(async move {
                let api = ApiClient::new();
                match api.break_start(&att_id).await {
                    Ok(_) => match refresh_state().await {
                        Ok(_) => set_message.set(Some("休憩を開始しました。".into())),
                        Err(err) => {
                            log::error!("Failed to refresh attendance context: {}", err);
                            set_message.set(Some(format!("状態の更新に失敗しました: {}", err)));
                        }
                    },
                    Err(err) => set_message.set(Some(format!("休憩開始に失敗しました: {}", err))),
                };
                set_loading.set(false);
            });
        }
    };

    let handle_break_end = {
        let attendance_state = attendance_state.clone();
        let set_message = set_message.clone();
        move |_| {
            if loading.get() {
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
            set_loading.set(true);
            set_message.set(None);
            let set_message = set_message.clone();
            let set_loading = set_loading.clone();
            spawn_local(async move {
                let api = ApiClient::new();
                match api.break_end(&break_id).await {
                    Ok(_) => match refresh_state().await {
                        Ok(_) => set_message.set(Some("休憩を終了しました。".into())),
                        Err(err) => {
                            log::error!("Failed to refresh attendance context: {}", err);
                            set_message.set(Some(format!("状態の更新に失敗しました: {}", err)));
                        }
                    },
                    Err(err) => set_message.set(Some(format!("休憩終了に失敗しました: {}", err))),
                };
                set_loading.set(false);
            });
        }
    };

    let handle_clock_out = {
        let set_attendance_state = set_attendance_state.clone();
        let set_message = set_message.clone();
        let set_loading = set_loading.clone();
        move |_| {
            if loading.get() {
                return;
            }
            set_loading.set(true);
            set_message.set(None);
            let set_state = set_attendance_state.clone();
            let set_message = set_message.clone();
            let set_loading = set_loading.clone();
            spawn_local(async move {
                match attendance_state::clock_out(set_state.clone()).await {
                    Ok(_) => match refresh_state().await {
                        Ok(_) => set_message.set(Some("退勤しました。".into())),
                        Err(err) => {
                            log::error!("Failed to refresh attendance context: {}", err);
                            set_message.set(Some(format!("状態の更新に失敗しました: {}", err)));
                        }
                    },
                    Err(err) => set_message.set(Some(format!("退勤に失敗しました: {}", err))),
                };
                set_loading.set(false);
            });
        }
    };

    let status_snapshot = move || attendance_state.get().today_status.clone();
    let holiday_reason = create_memo(move |_| attendance_state.get().today_holiday_reason.clone());
    let button_state = move || {
        let flags = button_flags_for(
            status_snapshot().as_ref().map(|s| s.status.as_str()),
            loading.get(),
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
                        <span class=format!("inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {}", color)>
                            {label}
                        </span>
                    }.into_view()
                }}
                <div class="flex flex-wrap gap-3">
                    <button
                        class="px-4 py-2 rounded bg-indigo-600 text-white disabled:opacity-50"
                        disabled={move || !button_state().clock_in}
                        on:click=handle_clock_in
                    >{"出勤"}</button>
                    <button
                        class="px-4 py-2 rounded bg-amber-600 text-white disabled:opacity-50"
                        disabled={move || !button_state().break_start}
                        on:click=handle_break_start
                    >{"休憩開始"}</button>
                    <button
                        class="px-4 py-2 rounded bg-amber-700 text-white disabled:opacity-50"
                        disabled={move || !button_state().break_end}
                        on:click=handle_break_end
                    >{"休憩終了"}</button>
                    <button
                        class="px-4 py-2 rounded bg-red-600 text-white disabled:opacity-50"
                        disabled={move || !button_state().clock_out}
                        on:click=handle_clock_out
                    >{"退勤"}</button>
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
            {move || {
                message
                    .get()
                    .map(|msg| view! { <p class="text-sm text-gray-700">{msg}</p> }.into_view())
                    .unwrap_or_else(|| view! {}.into_view())
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn not_started_only_allows_clock_in() {
        let flags = button_flags_for(None, false);
        assert!(flags.clock_in);
        assert!(!flags.break_start);
        assert!(!flags.break_end);
        assert!(!flags.clock_out);
    }

    #[wasm_bindgen_test]
    fn clocked_in_allows_break_start_and_clock_out() {
        let flags = button_flags_for(Some("clocked_in"), false);
        assert!(!flags.clock_in);
        assert!(flags.break_start);
        assert!(!flags.break_end);
        assert!(flags.clock_out);
    }

    #[wasm_bindgen_test]
    fn on_break_allows_break_end_and_clock_out() {
        let flags = button_flags_for(Some("on_break"), false);
        assert!(!flags.clock_in);
        assert!(!flags.break_start);
        assert!(flags.break_end);
        assert!(flags.clock_out);
    }

    #[wasm_bindgen_test]
    fn clocked_out_disables_all_buttons() {
        let flags = button_flags_for(Some("clocked_out"), false);
        assert_eq!(flags, AttendanceButtonFlags::default());
    }

    #[wasm_bindgen_test]
    fn loading_state_disables_everything() {
        let flags = button_flags_for(Some("clocked_in"), true);
        assert_eq!(flags, AttendanceButtonFlags::default());
    }
}
