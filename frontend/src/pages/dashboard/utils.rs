use crate::utils::time::now_in_app_tz;
use chrono::Datelike;

pub fn current_year_month() -> (i32, u32) {
    let now = now_in_app_tz();
    (now.year(), now.month())
}

pub fn format_hours(hours: Option<f64>) -> String {
    hours
        .map(|h| format!("{:.2} {}", h, rust_i18n::t!("common.units.hours")))
        .unwrap_or_else(|| "-".into())
}

pub fn format_days(days: Option<i32>) -> String {
    days.map(|days| {
        let unit_key = if days == 1 {
            "common.units.day"
        } else {
            "common.units.days"
        };
        format!("{days} {}", rust_i18n::t!(unit_key))
    })
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
    use crate::test_support::helpers::set_test_locale;

    #[test]
    fn formats_hours_with_two_decimals() {
        let _locale = set_test_locale("en");
        assert_eq!(format_hours(Some(12.3456)), "12.35 hr");
        assert_eq!(format_hours(Some(0.0)), "0.00 hr");
        assert_eq!(format_hours(None), "-");
    }

    #[test]
    fn formats_days_with_suffix() {
        let _locale = set_test_locale("en");
        assert_eq!(format_days(Some(1)), "1 day");
        assert_eq!(format_days(Some(5)), "5 days");
        assert_eq!(format_days(None), "-");
    }
}
