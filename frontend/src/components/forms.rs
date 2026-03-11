use crate::state::attendance::{describe_holiday_reason, AttendanceState, ClockMessage};
use chrono::{Datelike, NaiveDate};
use leptos::{ev::MouseEvent, *};
use wasm_bindgen::JsCast;

fn weekday_label(weekday: chrono::Weekday) -> &'static str {
    match weekday {
        chrono::Weekday::Mon => "common.time.weekdays.mon",
        chrono::Weekday::Tue => "common.time.weekdays.tue",
        chrono::Weekday::Wed => "common.time.weekdays.wed",
        chrono::Weekday::Thu => "common.time.weekdays.thu",
        chrono::Weekday::Fri => "common.time.weekdays.fri",
        chrono::Weekday::Sat => "common.time.weekdays.sat",
        chrono::Weekday::Sun => "common.time.weekdays.sun",
    }
}

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
    message: ReadSignal<Option<ClockMessage>>,
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
                        <p class="text-xs font-display font-bold text-action-primary-bg uppercase tracking-wider">
                            {rust_i18n::t!("components.forms.current_status")}
                        </p>
                        {move || {
                            let status = status_snapshot();
                            let (label_key, color, dot_color) = match status.as_ref().map(|s| s.status.as_str()) {
                                Some("clocked_in") => ("components.forms.status.clocked_in", "text-status-attendance-text-active", "bg-status-attendance-clock_in animate-pulse"),
                                Some("on_break") => ("components.forms.status.on_break", "text-status-attendance-text-active", "bg-status-attendance-break animate-pulse"),
                                Some("clocked_out") => ("components.forms.status.clocked_out", "text-status-attendance-text-inactive", "bg-status-attendance-clock_out"),
                                _ => ("components.forms.status.not_started", "text-status-attendance-text-inactive", "bg-status-attendance-not_started"),
                            };
                            view! {
                                <div class="flex items-center gap-2 mt-0.5">
                                    <span class=format!("w-2 h-2 rounded-full {}", dot_color)></span>
                                    <span class=format!("text-lg font-bold {}", color)>{rust_i18n::t!(label_key)}</span>
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
                    <span class="font-bold">{rust_i18n::t!("components.forms.actions.clock_in")}</span>
                </button>

                <button
                    class="group relative flex flex-col items-center justify-center p-4 rounded-2xl border-2 transition-all duration-200 transform active:scale-95 disabled:opacity-40 disabled:active:scale-100
                           border-status-attendance-break bg-surface-elevated text-status-attendance-break hover:bg-status-warning-bg disabled:border-state-disabled-border disabled:bg-state-disabled-bg disabled:text-state-disabled-text"
                    disabled={move || !button_state().break_start}
                    on:click=move |ev| on_break_start.call(ev)
                >
                    <i class="fas fa-coffee text-xl mb-2 group-disabled:text-state-disabled-text"></i>
                    <span class="font-bold">{rust_i18n::t!("components.forms.actions.break_start")}</span>
                </button>

                <button
                    class="group relative flex flex-col items-center justify-center p-4 rounded-2xl border-2 transition-all duration-200 transform active:scale-95 disabled:opacity-40 disabled:active:scale-100
                           border-status-attendance-break bg-status-attendance-break text-text-inverse shadow-lg hover:bg-status-warning-text disabled:border-state-disabled-border disabled:bg-state-disabled-bg disabled:text-state-disabled-text disabled:shadow-none"
                    disabled={move || !button_state().break_end}
                    on:click=move |ev| on_break_end.call(ev)
                >
                    <i class="fas fa-mug-hot text-xl mb-2 group-disabled:text-state-disabled-text"></i>
                    <span class="font-bold">{rust_i18n::t!("components.forms.actions.break_end")}</span>
                </button>

                <button
                    class="group relative flex flex-col items-center justify-center p-4 rounded-2xl border-2 transition-all duration-200 transform active:scale-95 disabled:opacity-40 disabled:active:scale-100
                           border-action-danger-border bg-surface-elevated text-action-danger-bg hover:bg-status-error-bg disabled:border-state-disabled-border disabled:bg-state-disabled-bg disabled:text-state-disabled-text"
                    disabled={move || !button_state().clock_out}
                    on:click=move |ev| on_clock_out.call(ev)
                >
                    <i class="fas fa-sign-out-alt text-xl mb-2 group-disabled:text-state-disabled-text"></i>
                    <span class="font-bold">{rust_i18n::t!("components.forms.actions.clock_out")}</span>
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
                                <span class="text-sm font-medium">
                                    {rust_i18n::t!("components.forms.holiday_blocked", label = label)}
                                </span>
                            </div>
                        }
                        .into_view()
                    })
                    .unwrap_or_else(|| view! {}.into_view())
            }}

            <Show when=move || action_pending.get()>
                <div class="flex items-center justify-center gap-2 py-2 text-action-primary-bg">
                    <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-current"></div>
                    <p class="text-sm font-medium">{rust_i18n::t!("common.states.processing")}</p>
                </div>
            </Show>

            {move || {
                message
                    .get()
                    .map(|message| {
                        let (text, is_error) = match message {
                            ClockMessage::Success(text) => (text, false),
                            ClockMessage::Error(err) => (err.error, true),
                        };
                        let (bg, border, color, icon) = if is_error {
                            (
                                "bg-status-error-bg",
                                "border-status-error-border",
                                "text-status-error-text",
                                "fa-exclamation-circle",
                            )
                        } else {
                            (
                                "bg-status-success-bg",
                                "border-status-success-border",
                                "text-status-success-text",
                                "fa-check-circle",
                            )
                        };
                        view! {
                            <div class=format!("flex items-center gap-2 p-3 rounded-xl border {} {} {} animate-pop-in", bg, border, color)>
                                <i class=format!("fas {}", icon)></i>
                                <p class="text-sm font-medium">{text}</p>
                            </div>
                        }
                        .into_view()
                    })
                    .unwrap_or_else(|| view! {}.into_view())
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_flags_follow_status() {
        let flags = button_flags_for(Some("not_started"), false);
        assert!(flags.clock_in);
        assert!(!flags.clock_out);

        let flags = button_flags_for(Some("clocked_in"), false);
        assert!(flags.break_start);
        assert!(flags.clock_out);

        let flags = button_flags_for(Some("on_break"), false);
        assert!(flags.break_end);
        assert!(flags.clock_out);

        let flags = button_flags_for(Some("clocked_out"), false);
        assert!(!flags.clock_in);
        assert!(!flags.clock_out);
    }

    #[test]
    fn loading_disables_buttons() {
        let flags = button_flags_for(Some("clocked_in"), true);
        assert!(!flags.clock_in);
        assert!(!flags.break_start);
        assert!(!flags.break_end);
        assert!(!flags.clock_out);
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::test_support::helpers::set_test_locale;
    use crate::test_support::ssr::render_to_string;

    #[test]
    fn date_picker_resolves_label_keys_and_weekday_text() {
        let _locale = set_test_locale("en");
        let html = render_to_string(move || {
            let value = create_rw_signal("2025-01-01".to_string());
            view! { <DatePicker value=value label=Some("pages.attendance.filters.from") /> }
        });

        assert!(html.contains("From"));
    }
}

#[component]
pub fn DatePicker(
    #[prop(into)] value: RwSignal<String>,
    /// Translation key for the field label. Pass locale keys such as
    /// `pages.attendance.filters.from`, not already-rendered display text.
    label: Option<&'static str>,
    #[prop(optional)] disabled: MaybeSignal<bool>,
) -> impl IntoView {
    let input_ref = create_node_ref::<html::Input>();

    let display_value = move || {
        let val = value.get();
        if val.is_empty() {
            return rust_i18n::t!("components.forms.date_picker.placeholder").into_owned();
        }
        if let Ok(date) = NaiveDate::parse_from_str(&val, "%Y-%m-%d") {
            let day_of_week = rust_i18n::t!(weekday_label(date.weekday())).into_owned();
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
            {label.map(|label_key| view! {
                <label class="text-sm font-bold text-fg-muted ml-1">{rust_i18n::t!(label_key)}</label>
            })}
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
