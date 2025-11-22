use chrono::{Datelike, NaiveDate};

fn is_holiday_or_weekend(date: NaiveDate, holidays: &[NaiveDate]) -> bool {
    date.weekday().number_from_monday() >= 6 || holidays.contains(&date)
}

#[test]
fn detects_weekend_without_db() {
    let saturday = NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(); // Saturday
    assert!(is_holiday_or_weekend(saturday, &[]));
}

#[test]
fn detects_configured_holiday_without_db() {
    let holiday = NaiveDate::from_ymd_opt(2025, 3, 4).unwrap();
    assert!(is_holiday_or_weekend(holiday, &[holiday]));
}
