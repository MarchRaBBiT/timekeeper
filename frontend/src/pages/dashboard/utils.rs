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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivityStatusFilter {
    All,
    PendingOnly,
    ApprovedOnly,
}

impl ActivityStatusFilter {
    pub fn from_str(value: &str) -> Self {
        match value {
            "pending" => ActivityStatusFilter::PendingOnly,
            "approved" => ActivityStatusFilter::ApprovedOnly,
            _ => ActivityStatusFilter::All,
        }
    }

    pub fn as_value(self) -> &'static str {
        match self {
            ActivityStatusFilter::All => "all",
            ActivityStatusFilter::PendingOnly => "pending",
            ActivityStatusFilter::ApprovedOnly => "approved",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_hours_with_two_decimals() {
        assert_eq!(format_hours(Some(12.3456)), "12.35時間");
        assert_eq!(format_hours(Some(0.0)), "0.00時間");
        assert_eq!(format_hours(None), "-");
    }

    #[test]
    fn formats_days_with_suffix() {
        assert_eq!(format_days(Some(5)), "5 日");
        assert_eq!(format_days(None), "-");
    }
}
