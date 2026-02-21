//! Common validation rules shared across request payloads.

use validator::ValidationError;

/// Validates username format.
///
/// Requirements:
/// - Only alphanumeric characters and underscores
/// - 1-50 characters in length
pub fn validate_username(username: &str) -> Result<(), ValidationError> {
    if username.is_empty() || username.len() > 50 {
        return Err(ValidationError::new("username_invalid_length"));
    }

    if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(ValidationError::new("username_invalid_characters"));
    }

    Ok(())
}

/// Validates that planned hours are within acceptable range.
///
/// Requirements:
/// - Between 0.5 and 24.0 hours
#[allow(dead_code)]
pub fn validate_planned_hours(hours: f64) -> Result<(), ValidationError> {
    if !(0.5..=24.0).contains(&hours) {
        return Err(ValidationError::new("planned_hours_out_of_range"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn username_rejects_empty() {
        let result = validate_username("");
        assert!(result.is_err());
    }

    #[test]
    fn username_rejects_special_chars() {
        let result = validate_username("user@name");
        assert!(result.is_err());
    }

    #[test]
    fn username_accepts_valid() {
        let result = validate_username("valid_user123");
        assert!(result.is_ok());
    }

    #[test]
    fn planned_hours_rejects_too_low() {
        let result = validate_planned_hours(0.25);
        assert!(result.is_err());
    }

    #[test]
    fn planned_hours_rejects_too_high() {
        let result = validate_planned_hours(25.0);
        assert!(result.is_err());
    }

    #[test]
    fn planned_hours_accepts_valid() {
        let result = validate_planned_hours(2.5);
        assert!(result.is_ok());
    }
}
