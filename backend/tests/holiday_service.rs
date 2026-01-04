use chrono::NaiveDate;
use timekeeper_backend::services::holiday::{HolidayReason, HolidayServiceStub, HolidayServiceTrait};

#[tokio::test]
async fn exception_override_beats_public_and_weekly_flags() {
    let public = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
    let weekly = NaiveDate::from_ymd_opt(2025, 4, 2).unwrap();
    let override_date = public;

    let service = HolidayServiceStub::new([public], [weekly], [(override_date, false)]).service();

    let decision = service
        .is_holiday(override_date, None)
        .await
        .expect("holiday decision");

    assert!(!decision.is_holiday);
    assert_eq!(decision.reason, HolidayReason::ExceptionOverride);
}

#[tokio::test]
async fn list_month_applies_service_logic_over_mixed_sources() {
    let public = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
    let weekly_first = NaiveDate::from_ymd_opt(2025, 4, 4).unwrap();
    let weekly_second = NaiveDate::from_ymd_opt(2025, 4, 11).unwrap();
    let forced_holiday = NaiveDate::from_ymd_opt(2025, 4, 15).unwrap();

    let service = HolidayServiceStub::new(
        [public],
        [weekly_first, weekly_second],
        [(forced_holiday, true)],
    )
    .service();

    let entries = service
        .list_month(2025, 4, None)
        .await
        .expect("list monthly holidays");
    let dates: Vec<_> = entries.iter().map(|entry| entry.date).collect();

    assert_eq!(
        dates,
        vec![public, weekly_first, weekly_second, forced_holiday]
    );
    assert!(entries.iter().all(|entry| matches!(
        entry.reason,
        HolidayReason::PublicHoliday
            | HolidayReason::WeeklyHoliday
            | HolidayReason::ExceptionOverride
    )));
}
