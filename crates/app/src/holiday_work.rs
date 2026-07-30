use chrono::{Months, NaiveDate};
use thiserror::Error;
use timekeeper_contract::holiday_work::{
    HolidayWorkBenefit, HolidayWorkRequestStatus, SubmitHolidayWorkRequest,
};

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum HolidayWorkError {
    #[error("substitution requires a distinct substitute_date")]
    InvalidSubstitution,
    #[error("compensatory leave requires positive compensatory_minutes")]
    InvalidCompensatoryMinutes,
    #[error("only pending requests can be decided")]
    NotPending,
    #[error("applicant cannot decide their own request")]
    SelfDecision,
    #[error("compensatory expiry configuration is missing")]
    ExpiryConfigurationMissing,
}

pub fn validate_submission(input: &SubmitHolidayWorkRequest) -> Result<(), HolidayWorkError> {
    match input.benefit {
        HolidayWorkBenefit::Substitution
            if input.substitute_date.is_none()
                || input.substitute_date == Some(input.work_date)
                || input.compensatory_minutes.is_some() =>
        {
            Err(HolidayWorkError::InvalidSubstitution)
        }
        HolidayWorkBenefit::Compensatory
            if input.substitute_date.is_some()
                || input
                    .compensatory_minutes
                    .is_none_or(|minutes| minutes <= 0) =>
        {
            Err(HolidayWorkError::InvalidCompensatoryMinutes)
        }
        _ => Ok(()),
    }
}

pub fn validate_decision(
    status: HolidayWorkRequestStatus,
    applicant_id: &str,
    actor_id: &str,
) -> Result<(), HolidayWorkError> {
    if status != HolidayWorkRequestStatus::Pending {
        return Err(HolidayWorkError::NotPending);
    }
    if applicant_id == actor_id {
        return Err(HolidayWorkError::SelfDecision);
    }
    Ok(())
}

pub fn compensatory_expiry(
    worked_on: NaiveDate,
    expiry_months: Option<u32>,
) -> Result<NaiveDate, HolidayWorkError> {
    let months = expiry_months
        .filter(|months| *months > 0)
        .ok_or(HolidayWorkError::ExpiryConfigurationMissing)?;
    worked_on
        .checked_add_months(Months::new(months))
        .ok_or(HolidayWorkError::ExpiryConfigurationMissing)
}
