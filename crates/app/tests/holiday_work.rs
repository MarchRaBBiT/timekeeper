use chrono::NaiveDate;
use timekeeper_app::holiday_work::{
    compensatory_expiry, validate_decision, validate_submission, HolidayWorkError,
};
use timekeeper_contract::holiday_work::{
    HolidayWorkBenefit, HolidayWorkRequestStatus, SubmitHolidayWorkRequest,
};

fn date(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 8, day).expect("date")
}

#[test]
fn benefit_choice_is_exclusive() {
    let invalid = SubmitHolidayWorkRequest {
        work_date: date(9),
        benefit: HolidayWorkBenefit::Substitution,
        substitute_date: Some(date(10)),
        compensatory_minutes: Some(480),
        reason: "invalid".into(),
    };
    assert_eq!(
        validate_submission(&invalid),
        Err(HolidayWorkError::InvalidSubstitution)
    );

    let invalid_compensatory = SubmitHolidayWorkRequest {
        work_date: date(9),
        benefit: HolidayWorkBenefit::Compensatory,
        substitute_date: Some(date(10)),
        compensatory_minutes: Some(0),
        reason: "invalid".into(),
    };
    assert_eq!(
        validate_submission(&invalid_compensatory),
        Err(HolidayWorkError::InvalidCompensatoryMinutes)
    );

    let valid = SubmitHolidayWorkRequest {
        work_date: date(9),
        benefit: HolidayWorkBenefit::Compensatory,
        substitute_date: None,
        compensatory_minutes: Some(480),
        reason: "worked holiday".into(),
    };
    assert_eq!(validate_submission(&valid), Ok(()));
}

#[test]
fn self_or_repeated_decision_is_rejected() {
    assert_eq!(
        validate_decision(HolidayWorkRequestStatus::Pending, "u1", "u1"),
        Err(HolidayWorkError::SelfDecision)
    );
    assert_eq!(
        validate_decision(HolidayWorkRequestStatus::Approved, "u1", "manager"),
        Err(HolidayWorkError::NotPending)
    );
    assert_eq!(
        validate_decision(HolidayWorkRequestStatus::Pending, "u1", "manager"),
        Ok(())
    );
}

#[test]
fn expiry_uses_effective_setting() {
    assert_eq!(
        compensatory_expiry(date(9), Some(2)).expect("expiry"),
        NaiveDate::from_ymd_opt(2026, 10, 9).expect("date")
    );
    assert_eq!(
        compensatory_expiry(date(9), None),
        Err(HolidayWorkError::ExpiryConfigurationMissing)
    );
    assert_eq!(
        compensatory_expiry(date(9), Some(0)),
        Err(HolidayWorkError::ExpiryConfigurationMissing)
    );
}
