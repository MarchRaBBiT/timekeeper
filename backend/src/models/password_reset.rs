//! Models for password reset functionality.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
pub use timekeeper_contract::auth::{
    MessageResponse, RequestPasswordResetRequest as RequestPasswordResetPayload,
    ResetPasswordRequest as ResetPasswordPayload,
};
use utoipa::ToSchema;

use crate::types::UserId;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
/// Database representation of a password reset token.
pub struct PasswordReset {
    /// Unique identifier for the password reset record.
    pub id: String,
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
