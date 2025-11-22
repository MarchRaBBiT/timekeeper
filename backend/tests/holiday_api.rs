use chrono::{Datelike, NaiveDate};
use timekeeper_backend::models::holiday::Holiday;

fn to_month(date: NaiveDate) -> (i32, u32) {
    (date.year(), date.month())
}

#[test]
fn month_filtering_without_db() {
    let dates = [
        NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(2025, 2, 15).unwrap(),
        NaiveDate::from_ymd_opt(2025, 1, 10).unwrap(),
    ];
    let jan_holidays: Vec<_> = dates
        .iter()
        .map(|d| Holiday::new(*d, "x".into(), None))
        .filter(|h| to_month(h.holiday_date) == (2025, 1))
        .collect();
    assert_eq!(jan_holidays.len(), 2);
}
