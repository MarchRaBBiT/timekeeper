//! Models for password reset functionality.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::types::UserId;
use crate::validation::rules;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
/// Database representation of a password reset token.
pub struct PasswordReset {
    /// Unique identifier for the password reset record.
    pub id: Uuid,
    /// User ID associated with this reset token.
    pub user_id: UserId,
    /// SHA-256 hash of the reset token (for security).
    pub token_hash: String,
    /// Timestamp when this token expires.
    pub expires_at: DateTime<Utc>,
    /// Creation timestamp for auditing.
    pub created_at: DateTime<Utc>,
    /// Timestamp when this token was used (null if not yet used).
    pub used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
/// Payload for requesting a password reset.
pub struct RequestPasswordResetPayload {
    /// Email address of the user requesting password reset.
    #[validate(email(message = "Invalid email address"))]
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
/// Payload for resetting password with a token.
pub struct ResetPasswordPayload {
    /// Password reset token from the email.
    #[validate(length(min = 32, message = "Invalid reset token"))]
    pub token: String,
    #[validate(custom(function = "rules::validate_password_strength"))]
    pub new_password: String,
}
