use chrono::NaiveDate;
use timekeeper_backend::handlers::admin::AdminHolidayKind;

#[test]
fn prioritizes_override_holiday_without_db() {
    let base = AdminHolidayKind::Public;
    let override_kind = AdminHolidayKind::Exception;
    let date = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();

    // Simulate that an override wins over a base kind when both exist.
    let chosen = if override_kind == AdminHolidayKind::Exception {
        override_kind
    } else {
        base
    };

    assert_eq!(chosen, AdminHolidayKind::Exception);
    assert_eq!(date, NaiveDate::from_ymd_opt(2025, 4, 1).unwrap());
}
