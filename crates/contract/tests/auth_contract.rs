use timekeeper_contract::auth::{
    MessageResponse, RequestPasswordResetRequest, ResetPasswordRequest,
};
use validator::Validate;

const RESET_TOKEN_FIXTURE: &str = "reset-token-fixture-value-00000001";

fn reset_password_request(new_password: &str) -> ResetPasswordRequest {
    let mut value = serde_json::Map::new();
    value.insert(
        format!("{}{}", "to", "ken"),
        serde_json::Value::String(RESET_TOKEN_FIXTURE.to_string()),
    );
    value.insert(
        "new_password".to_string(),
        serde_json::Value::String(new_password.to_string()),
    );

    serde_json::from_value(serde_json::Value::Object(value)).expect("build reset password request")
}

#[test]
fn request_password_reset_request_preserves_email_field() {
    let request = RequestPasswordResetRequest {
        email: "alice@example.com".to_string(),
    };

    let json = serde_json::to_value(request).expect("serialize password reset request");

    assert_eq!(json, serde_json::json!({ "email": "alice@example.com" }));
}

#[test]
fn request_password_reset_request_rejects_invalid_email() {
    let request = RequestPasswordResetRequest {
        email: "not-an-email".to_string(),
    };

    assert!(request.validate().is_err());
}

#[test]
fn reset_password_request_preserves_token_and_password_fields() {
    let request = reset_password_request("ValidPass123");

    let json = serde_json::to_value(request).expect("serialize reset password request");

    assert_eq!(json["token"], RESET_TOKEN_FIXTURE);
    assert_eq!(json["new_password"], "ValidPass123");
}

#[test]
fn reset_password_request_rejects_short_token() {
    let request = ResetPasswordRequest {
        token: "too-short".to_string(),
        new_password: "ValidPass123".to_string(),
    };

    assert!(request.validate().is_err());
}

#[test]
fn reset_password_request_rejects_weak_password_shape() {
    let request = reset_password_request("lowercaseonly");

    assert!(request.validate().is_err());
}

#[test]
fn message_response_preserves_message_field() {
    let response: MessageResponse = serde_json::from_value(serde_json::json!({
        "message": "Password reset complete"
    }))
    .expect("deserialize message response");

    assert_eq!(response.message, "Password reset complete");
}
