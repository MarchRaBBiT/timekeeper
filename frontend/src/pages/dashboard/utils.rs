use crate::utils::time::now_in_app_tz;
use chrono::Datelike;

pub fn current_year_month() -> (i32, u32) {
    let now = now_in_app_tz();
    (now.year(), now.month())
}

pub fn format_hours(hours: Option<f64>) -> String {
    hours
        .map(|h| format!("{:.2}時間", h))
        .unwrap_or_else(|| "-".into())
}

pub fn format_days(days: Option<i32>) -> String {
    days.map(|d| format!("{d} 日"))
        .unwrap_or_else(|| "-".into())
}
