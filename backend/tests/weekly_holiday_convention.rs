use chrono::NaiveDate;
use timekeeper_backend::services::holiday::{
    HolidayReason, HolidayServiceStub, HolidayServiceTrait,
};

#[tokio::test]
async fn sunday_is_index_zero_convention() {
    // 2025-12-28 is Sunday
    let sunday = NaiveDate::from_ymd_opt(2025, 12, 28).unwrap();
    // 2025-12-29 is Monday
    let monday = NaiveDate::from_ymd_opt(2025, 12, 29).unwrap();

    // If we define a weekly holiday with weekday=0 (Sunday in frontend/new convention)
    let service = HolidayServiceStub::new([], [sunday], []).service();

    // It should be a holiday on Sunday
    let sunday_decision = service.is_holiday(sunday, None).await.expect("decision");
    assert!(sunday_decision.is_holiday);
    assert_eq!(sunday_decision.reason, HolidayReason::WeeklyHoliday);

    // It should NOT be a holiday on Monday
    let monday_decision = service.is_holiday(monday, None).await.expect("decision");
    assert!(!monday_decision.is_holiday);
}
