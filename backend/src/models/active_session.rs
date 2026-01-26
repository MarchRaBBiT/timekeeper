//! Models for tracking active user sessions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

use crate::types::UserId;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
/// Database representation of an active user session.
pub struct ActiveSession {
    /// Unique identifier for the session record.
    pub id: String,
    /// User ID associated with the session.
    pub user_id: UserId,
    /// Refresh token ID linked to the session.
    pub refresh_token_id: String,
    /// Optional label identifying the client/device.
    pub device_label: Option<String>,
    /// Timestamp when the session was created.
    pub created_at: DateTime<Utc>,
    /// Timestamp when the session was last used.
    pub last_seen_at: Option<DateTime<Utc>>,
    /// Timestamp when the session expires.
    pub expires_at: DateTime<Utc>,
}
