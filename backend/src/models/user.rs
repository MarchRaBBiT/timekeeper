use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub full_name: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
pub enum UserRole {
    Employee,
    Admin,
}

impl Default for UserRole {
    fn default() -> Self {
        UserRole::Employee
    }
}

impl UserRole {
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

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUser {
    pub username: String,
    pub password: String,
    pub full_name: String,
    pub role: UserRole,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateUser {
    pub full_name: Option<String>,
    pub role: Option<UserRole>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub user: UserResponse,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub full_name: String,
    pub role: String,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        UserResponse {
            id: user.id,
            username: user.username,
            full_name: user.full_name,
            role: user.role.as_str().to_string(),
        }
    }
}

impl User {
    pub fn new(username: String, password_hash: String, full_name: String, role: UserRole) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            username,
            password_hash,
            full_name,
            role,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn is_admin(&self) -> bool {
        matches!(self.role, UserRole::Admin)
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
        );
        let resp: UserResponse = user.into();
        assert_eq!(resp.role, "admin");
    }
}
