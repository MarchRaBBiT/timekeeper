use gloo_timers::callback::Interval;
use leptos::*;

use crate::utils::time::now_in_app_tz;

#[component]
pub fn Clock() -> impl IntoView {
    let (time, set_time) = create_signal(now_in_app_tz());

    // Update time every second
    // We use store_value to keep the Interval alive. It will be dropped (and cancelled) when the component is unmounted.
    let _interval = store_value(Interval::new(1000, move || {
        set_time.set(now_in_app_tz());
    }));

    // Format date: "2024年12月30日(月)"
    let date_str = move || {
        let t = time.get();
        // Custom formatting for Japanese locale-like output if needed, or use chrono's formatting
        // %Y年%m月%d日(%a) w/ locale if possible, but chrono's localized formatting might need feature flags
        // For simplicity and robustness, specific format:
        format!("{}", t.format("%Y年%m月%d日"))
    };

    // Day of week in Japanese
    let weekday_str = move || {
        let t = time.get();
        match t.format("%A").to_string().as_str() {
            "Monday" => "(月)",
            "Tuesday" => "(火)",
            "Wednesday" => "(水)",
            "Thursday" => "(木)",
            "Friday" => "(金)",
            "Saturday" => "(土)",
            "Sunday" => "(日)",
            _ => "",
        }
    };

    // Format time: "12:34:56"
    let time_str = move || time.get().format("%H:%M:%S").to_string();

    view! {
        <div class="bg-gradient-to-br from-action-primary-bg to-action-primary-bg-hover text-text-inverse border-none shadow-lg rounded-lg overflow-hidden">
            <div class="flex flex-col items-center justify-center py-4 space-y-2">
                <div class="text-lg font-medium opacity-90">
                    {date_str} <span class="ml-1">{weekday_str}</span>
                </div>
                <div class="text-4xl font-bold tracking-wider font-mono">
                    {time_str}
                </div>
            </div>
        </div>
    }
}
