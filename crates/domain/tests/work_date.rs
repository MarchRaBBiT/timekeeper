use timekeeper_domain::WorkDate;

#[test]
fn work_date_accepts_real_calendar_dates() {
    let date = WorkDate::from_ymd(2026, 6, 12).expect("valid work date");

    assert_eq!(date.to_string(), "2026-06-12");
}

#[test]
fn work_date_rejects_invalid_calendar_dates() {
    let error = WorkDate::from_ymd(2026, 2, 30).expect_err("invalid work date");

    assert_eq!(error.to_string(), "invalid work date: 2026-02-30");
}
