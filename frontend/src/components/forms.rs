use crate::state::attendance::{describe_holiday_reason, AttendanceState};
use chrono::{Datelike, NaiveDate};
use leptos::{ev::MouseEvent, *};
use wasm_bindgen::JsCast;

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
    action_pending: ReadSignal<bool>,
    message: ReadSignal<Option<crate::api::ApiError>>,
    on_clock_in: Callback<MouseEvent>,
    on_clock_out: Callback<MouseEvent>,
    on_break_start: Callback<MouseEvent>,
    on_break_end: Callback<MouseEvent>,
) -> impl IntoView {
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
        <div class="space-y-6 animate-fade-in">
            <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 p-4 rounded-2xl bg-surface-muted border border-border">
                <div class="flex items-center gap-3">
                    <div class="w-10 h-10 flex items-center justify-center rounded-xl bg-surface-elevated shadow-sm text-action-primary-bg">
                        <i class="fas fa-user-clock text-lg"></i>
                    </div>
                    <div>
                        <p class="text-xs font-display font-bold text-action-primary-bg uppercase tracking-wider">{"現在のステータス"}</p>
                        {move || {
                            let status = status_snapshot();
                            let (label, color, dot_color) = match status.as_ref().map(|s| s.status.as_str()) {
                                Some("clocked_in") => ("勤務中", "text-status-attendance-text-active", "bg-status-attendance-clock_in animate-pulse"),
                                Some("on_break") => ("休憩中", "text-status-attendance-text-active", "bg-status-attendance-break animate-pulse"),
                                Some("clocked_out") => ("退勤済み", "text-status-attendance-text-inactive", "bg-status-attendance-clock_out"),
                                _ => ("未出勤", "text-status-attendance-text-inactive", "bg-status-attendance-not_started"),
                            };
                            view! {
                                <div class="flex items-center gap-2 mt-0.5">
                                    <span class=format!("w-2 h-2 rounded-full {}", dot_color)></span>
                                    <span class=format!("text-lg font-bold {}", color)>{label}</span>
                                </div>
                            }.into_view()
                        }}
                    </div>
                </div>
            </div>

            <div class="grid grid-cols-2 gap-3">
                <button
                    class="group relative flex flex-col items-center justify-center p-4 rounded-2xl border-2 transition-all duration-200 transform active:scale-95 disabled:opacity-40 disabled:active:scale-100
                           border-status-attendance-clock_in bg-status-attendance-clock_in text-text-inverse shadow-lg hover:bg-action-primary-bg-hover hover:border-action-primary-border-hover disabled:border-state-disabled-border disabled:bg-state-disabled-bg disabled:text-state-disabled-text disabled:shadow-none"
                    disabled={move || !button_state().clock_in}
                    on:click=move |ev| on_clock_in.call(ev)
                >
                    <i class="fas fa-sign-in-alt text-xl mb-2 group-disabled:text-state-disabled-text"></i>
                    <span class="font-bold">{"出勤"}</span>
                </button>

                <button
                    class="group relative flex flex-col items-center justify-center p-4 rounded-2xl border-2 transition-all duration-200 transform active:scale-95 disabled:opacity-40 disabled:active:scale-100
                           border-status-attendance-break bg-surface-elevated text-status-attendance-break hover:bg-status-warning-bg disabled:border-state-disabled-border disabled:bg-state-disabled-bg disabled:text-state-disabled-text"
                    disabled={move || !button_state().break_start}
                    on:click=move |ev| on_break_start.call(ev)
                >
                    <i class="fas fa-coffee text-xl mb-2 group-disabled:text-state-disabled-text"></i>
                    <span class="font-bold">{"休憩開始"}</span>
                </button>

                <button
                    class="group relative flex flex-col items-center justify-center p-4 rounded-2xl border-2 transition-all duration-200 transform active:scale-95 disabled:opacity-40 disabled:active:scale-100
                           border-status-attendance-break bg-status-attendance-break text-text-inverse shadow-lg hover:bg-status-warning-text disabled:border-state-disabled-border disabled:bg-state-disabled-bg disabled:text-state-disabled-text disabled:shadow-none"
                    disabled={move || !button_state().break_end}
                    on:click=move |ev| on_break_end.call(ev)
                >
                    <i class="fas fa-mug-hot text-xl mb-2 group-disabled:text-state-disabled-text"></i>
                    <span class="font-bold">{"休憩終了"}</span>
                </button>

                <button
                    class="group relative flex flex-col items-center justify-center p-4 rounded-2xl border-2 transition-all duration-200 transform active:scale-95 disabled:opacity-40 disabled:active:scale-100
                           border-action-danger-border bg-surface-elevated text-action-danger-bg hover:bg-status-error-bg disabled:border-state-disabled-border disabled:bg-state-disabled-bg disabled:text-state-disabled-text"
                    disabled={move || !button_state().clock_out}
                    on:click=move |ev| on_clock_out.call(ev)
                >
                    <i class="fas fa-sign-out-alt text-xl mb-2 group-disabled:text-state-disabled-text"></i>
                    <span class="font-bold">{"退勤"}</span>
                </button>
            </div>

            {move || {
                holiday_reason
                    .get()
                    .map(|reason| {
                        let label = describe_holiday_reason(reason.trim());
                        view! {
                            <div class="flex items-center gap-3 p-4 rounded-2xl bg-status-warning-bg border border-status-warning-border text-status-warning-text animate-pop-in">
                                <i class="fas fa-calendar-day text-status-warning-text text-xl"></i>
                                <span class="text-sm font-medium">{format!("本日は{}のため打刻できません。", label)}</span>
                            </div>
                        }
                        .into_view()
                    })
                    .unwrap_or_else(|| view! {}.into_view())
            }}

            <Show when=move || action_pending.get()>
                <div class="flex items-center justify-center gap-2 py-2 text-action-primary-bg">
                    <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-current"></div>
                    <p class="text-sm font-medium">{"処理中..."}</p>
                </div>
            </Show>

            {move || {
                message
                    .get()
                    .map(|err| {
                        let msg = &err.error;
                        let is_success_message =
                            msg.ends_with("しました。") || msg.ends_with("完了しました。");
                        let is_error = match err.code.as_str() {
                            "REQUEST_FAILED" | "UNKNOWN" => true,
                            "VALIDATION_ERROR" => !is_success_message,
                            _ => true,
                        };
                        let (bg, border, text, icon) = if is_error {
                            ("bg-status-error-bg", "border-status-error-border", "text-status-error-text", "fa-exclamation-circle")
                        } else {
                            ("bg-status-success-bg", "border-status-success-border", "text-status-success-text", "fa-check-circle")
                        };
                        view! {
                            <div class=format!("flex items-center gap-2 p-3 rounded-xl border {} {} {} animate-pop-in", bg, border, text)>
                                <i class=format!("fas {}", icon)></i>
                                <p class="text-sm font-medium">{msg.clone()}</p>
                            </div>
                        }.into_view()
                    })
                    .unwrap_or_else(|| view! {}.into_view())
            }}
        </div>
    }
}

#[component]
pub fn DatePicker(
    #[prop(into)] value: RwSignal<String>,
    label: Option<&'static str>,
    #[prop(optional)] disabled: MaybeSignal<bool>,
) -> impl IntoView {
    let input_ref = create_node_ref::<html::Input>();

    let display_value = move || {
        let val = value.get();
        if val.is_empty() {
            return "日付を選択".to_string();
        }
        if let Ok(date) = NaiveDate::parse_from_str(&val, "%Y-%m-%d") {
            let day_of_week = match date.weekday() {
                chrono::Weekday::Mon => "月",
                chrono::Weekday::Tue => "火",
                chrono::Weekday::Wed => "水",
                chrono::Weekday::Thu => "木",
                chrono::Weekday::Fri => "金",
                chrono::Weekday::Sat => "土",
                chrono::Weekday::Sun => "日",
            };
            format!("{} ({})", date.format("%Y/%m/%d"), day_of_week)
        } else {
            val
        }
    };

    let on_click = move |_| {
        if disabled.get() {
            return;
        }
        if let Some(input) = input_ref.get() {
            // Try showPicker()
            let _ = js_sys::Reflect::get(&input, &"showPicker".into()).map(|f| {
                if f.is_function() {
                    let _ = js_sys::Reflect::apply(
                        &f.unchecked_into::<js_sys::Function>(),
                        &input,
                        &js_sys::Array::new(),
                    );
                }
            });
            // Always focus as fallback/additional trigger
            let _ = input.focus();
        }
    };

    view! {
        <div class="flex flex-col gap-1.5 w-full">
            {label.map(|l| view! { <label class="text-sm font-bold text-fg-muted ml-1">{l}</label> })}
            <div
                class=move || format!(
                    "relative group cursor-pointer rounded-xl border-2 transition-all duration-200 bg-form-control-bg py-2.5 px-4 flex items-center justify-between shadow-sm border-form-control-border hover:border-action-primary-border-hover hover:shadow-md active:scale-[0.98] {}",
                    if disabled.get() { "opacity-50 cursor-not-allowed bg-state-disabled-bg border-state-disabled-border text-state-disabled-text shadow-none touch-none" } else { "hover:ring-4 hover:ring-action-primary-focus" }
                )
                on:click=on_click
            >
                <div class="flex items-center gap-3">
                    <div class=move || format!(
                        "w-8 h-8 rounded-lg flex items-center justify-center transition-colors {}",
                        if value.get().is_empty() { "bg-surface-muted text-text-muted" } else { "bg-primary-subtle text-action-primary-bg" }
                    )>
                        <i class="far fa-calendar-alt text-base"></i>
                    </div>
                    <span class=move || format!(
                        "text-sm font-semibold tracking-wide {}",
                        if value.get().is_empty() { "text-text-muted" } else { "text-fg" }
                    )>
                        {display_value}
                    </span>
                </div>
                <div class="text-fg-muted group-hover:text-action-primary-bg transition-colors">
                    <i class="fas fa-chevron-down text-xs"></i>
                </div>

                // Hidden native input
                <input
                    type="date"
                    node_ref=input_ref
                    class="absolute inset-0 w-full h-full opacity-0 pointer-events-none"
                    disabled=disabled
                    prop:value={move || value.get()}
                    on:input=move |ev| value.set(event_target_value(&ev))
                />
            </div>
        </div>
    }
}
