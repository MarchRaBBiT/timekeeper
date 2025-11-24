use chrono::{Datelike, NaiveDate};

fn is_holiday_or_weekend(
    date: NaiveDate,
    holidays: &[NaiveDate],
    overrides: &[(NaiveDate, bool)],
) -> bool {
    if let Some((_, override_workday)) = overrides.iter().find(|(d, _)| *d == date) {
        return !override_workday;
    }
    date.weekday().number_from_monday() >= 6 || holidays.contains(&date)
}

#[test]
fn detects_weekend_without_db() {
    let saturday = NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(); // Saturday
    assert!(is_holiday_or_weekend(saturday, &[], &[]));
}

#[test]
fn detects_configured_holiday_without_db() {
    let holiday = NaiveDate::from_ymd_opt(2025, 3, 4).unwrap();
    assert!(is_holiday_or_weekend(holiday, &[holiday], &[]));
}

#[test]
fn override_can_cancel_holiday_without_db() {
    let holiday = NaiveDate::from_ymd_opt(2025, 3, 4).unwrap();
    assert!(
        !is_holiday_or_weekend(holiday, &[holiday], &[(holiday, true)]),
        "override=true should mark as working day"
    );
}
