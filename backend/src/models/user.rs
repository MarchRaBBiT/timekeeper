//! Models that represent users, authentication payloads, and role metadata.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
/// Database representation of an authenticated user account.
pub struct User {
    /// Unique identifier for the user.
    pub id: String,
    /// Immutable username used for login.
    pub username: String,
    /// Argon2/Bcrypt/Scrypt hash of the user's password.
    pub password_hash: String,
    /// Human-readable full name.
    pub full_name: String,
    /// Role describing the user's privileges.
    pub role: UserRole,
    /// Flag promoting the user to the highest administrative tier.
    pub is_system_admin: bool,
    /// Shared secret for RFC6238 TOTP verification (base32 encoded).
    pub mfa_secret: Option<String>,
    /// Timestamp marking when the user completed MFA enrollment.
    pub mfa_enabled_at: Option<DateTime<Utc>>,
    /// Creation timestamp for auditing.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp for auditing.
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::Type, ToSchema, Default)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
/// Supported user roles stored in the database.
pub enum UserRole {
    /// Standard employee role with limited permissions.
    #[default]
    Employee,
    /// Administrator role with elevated permissions.
    Admin,
}

impl UserRole {
    /// Returns the canonical snake_case representation of the role.
    pub fn as_str(&self) -> &'static str {
        match self {
            UserRole::Employee => "employee",
            UserRole::Admin => "admin",
        }
    }
}

impl Serialize for UserRole {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for UserRole {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            // primary canonical values (snake_case)
            "employee" => Ok(UserRole::Employee),
            "admin" => Ok(UserRole::Admin),
            // tolerate common legacy casings
            "Employee" | "EMPLOYEE" => Ok(UserRole::Employee),
            "Admin" | "ADMIN" => Ok(UserRole::Admin),
            other => Err(serde::de::Error::unknown_variant(
                other,
                &["employee", "admin"],
            )),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
/// Payload for creating a new user account.
pub struct CreateUser {
    pub username: String,
    pub password: String,
    pub full_name: String,
    pub role: UserRole,
    #[serde(default)]
    pub is_system_admin: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
/// Payload for updating portions of an existing user.
pub struct UpdateUser {
    pub full_name: Option<String>,
    pub role: Option<UserRole>,
    pub is_system_admin: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
/// Credentials submitted by a user attempting to authenticate.
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    /// Optional TOTP code required when MFA is enabled.
    #[serde(default)]
    pub totp_code: Option<String>,
    /// Optional label to identify the client/device for long-lived tokens.
    #[serde(default)]
    pub device_label: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
/// Payload submitted when a user requests to change their password.
pub struct ChangePasswordRequest {
    /// Existing password that will be verified before applying the change.
    pub current_password: String,
    /// Replacement password that will be stored if verification succeeds.
    pub new_password: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
/// Authentication tokens returned after a successful login.
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub user: UserResponse,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
/// Response returned when initiating MFA setup.
pub struct MfaSetupResponse {
    pub secret: String,
    pub otpauth_url: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
/// Request payload carrying a one-time MFA code.
pub struct MfaCodeRequest {
    pub code: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
/// Exposes MFA enrollment status for the current user.
pub struct MfaStatusResponse {
    pub enabled: bool,
    pub pending: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
/// Public-facing representation of a user returned by the API.
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub full_name: String,
    pub role: String,
    pub is_system_admin: bool,
    pub mfa_enabled: bool,
}

impl From<User> for UserResponse {
    /// Converts the persistent user model into the API response DTO.
    fn from(user: User) -> Self {
        let mfa_enabled = user.is_mfa_enabled();
        UserResponse {
            id: user.id,
            username: user.username,
            full_name: user.full_name,
            role: user.role.as_str().to_string(),
            is_system_admin: user.is_system_admin,
            mfa_enabled,
        }
    }
}

impl User {
    /// Constructs a new user with freshly generated identifiers.
    pub fn new(
        username: String,
        password_hash: String,
        full_name: String,
        role: UserRole,
        is_system_admin: bool,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            username,
            password_hash,
            full_name,
            role,
            is_system_admin,
            mfa_secret: None,
            mfa_enabled_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Returns `true` when the user holds the `Admin` role.
    pub fn is_admin(&self) -> bool {
        matches!(self.role, UserRole::Admin)
    }

    /// Returns `true` when the user is flagged as a system administrator.
    pub fn is_system_admin(&self) -> bool {
        self.is_system_admin
    }

    /// Returns `true` when the user has completed MFA enrollment.
    pub fn is_mfa_enabled(&self) -> bool {
        self.mfa_secret.is_some() && self.mfa_enabled_at.is_some()
    }

    /// Returns `true` when setup has been started but not yet confirmed.
    pub fn has_pending_mfa(&self) -> bool {
        self.mfa_secret.is_some() && self.mfa_enabled_at.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn user_role_serde_accepts_and_emits_snake_case() {
        // Accept snake_case
        let e: UserRole = serde_json::from_str("\"employee\"").unwrap();
        let a: UserRole = serde_json::from_str("\"admin\"").unwrap();
        assert!(matches!(e, UserRole::Employee));
        assert!(matches!(a, UserRole::Admin));

        // Tolerate legacy casings
        let e2: UserRole = serde_json::from_str("\"Employee\"").unwrap();
        let a2: UserRole = serde_json::from_str("\"ADMIN\"").unwrap();
        assert!(matches!(e2, UserRole::Employee));
        assert!(matches!(a2, UserRole::Admin));

        // Emit snake_case
        let se = serde_json::to_value(UserRole::Employee).unwrap();
        let sa = serde_json::to_value(UserRole::Admin).unwrap();
        assert_eq!(se, Value::String("employee".into()));
        assert_eq!(sa, Value::String("admin".into()));
    }

    #[test]
    fn user_response_role_is_snake_case_string() {
        let user = User::new(
            "alice".to_string(),
            "hash".to_string(),
            "Alice Example".to_string(),
            UserRole::Admin,
            true,
        );
        let resp: UserResponse = user.into();
        assert_eq!(resp.role, "admin");
        assert!(resp.is_system_admin);
        assert!(!resp.mfa_enabled);
    }

    #[test]
    fn update_user_payload_supports_optional_fields() {
        let update = UpdateUser {
            full_name: Some("Alice Example".into()),
            role: Some(UserRole::Admin),
            is_system_admin: Some(true),
        };
        assert_eq!(update.full_name.as_deref(), Some("Alice Example"));
        assert!(matches!(update.role, Some(UserRole::Admin)));
        assert_eq!(update.is_system_admin, Some(true));
    }
}
