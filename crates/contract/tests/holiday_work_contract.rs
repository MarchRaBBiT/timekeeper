use chrono::NaiveDate;
use timekeeper_contract::holiday_work::{HolidayWorkBenefit, SubmitHolidayWorkRequest};
use validator::Validate;

#[test]
fn substitution_contract_round_trips_snake_case() {
    let value = serde_json::json!({
        "work_date": "2026-08-09",
        "benefit": "substitution",
        "substitute_date": "2026-08-10",
        "compensatory_minutes": null,
        "reason": "事前振替"
    });
    let request: SubmitHolidayWorkRequest = serde_json::from_value(value).expect("deserialize");
    assert_eq!(request.benefit, HolidayWorkBenefit::Substitution);
    assert_eq!(
        request.substitute_date,
        NaiveDate::from_ymd_opt(2026, 8, 10)
    );
}

#[test]
fn compensatory_minutes_are_bounded() {
    let request = SubmitHolidayWorkRequest {
        work_date: NaiveDate::from_ymd_opt(2026, 8, 9).expect("date"),
        benefit: HolidayWorkBenefit::Compensatory,
        substitute_date: None,
        compensatory_minutes: Some(1441),
        reason: "代休".into(),
    };
    assert!(request.validate().is_err());
}
