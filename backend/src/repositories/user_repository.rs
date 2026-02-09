//! User repository trait for dependency injection and testing.
//!
//! This module defines the UserRepository trait which can be mocked
//! using mockall for testing purposes.

use async_trait::async_trait;
use sqlx::PgPool;

use crate::error::AppError;
use crate::models::user::User;
use crate::types::UserId;

/// Repository trait for User operations.
///
/// This trait is designed to be mockable using mockall for testing.
/// Use `MockUserRepository` in tests to mock the behavior.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
#[allow(dead_code)]
pub trait UserRepository: Send + Sync {
    /// Find all users
    async fn find_all(&self, db: PgPool) -> Result<Vec<User>, AppError>;

    /// Find a user by ID
    async fn find_by_id(&self, db: PgPool, id: UserId) -> Result<User, AppError>;

    /// Create a new user
    async fn create(&self, db: PgPool, user: &User) -> Result<User, AppError>;

    /// Update an existing user
    async fn update(&self, db: PgPool, user: &User) -> Result<User, AppError>;

    /// Delete a user by ID
    async fn delete(&self, db: PgPool, id: UserId) -> Result<(), AppError>;

    /// Find user by username
    async fn find_by_username(&self, db: PgPool, username: &str) -> Result<Option<User>, AppError>;

    /// Find user by email
    async fn find_by_email(&self, db: PgPool, email: &str) -> Result<Option<User>, AppError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_user_repository_can_be_created() {
        let _mock = MockUserRepository::new();
    }

    #[test]
    fn test_mock_user_repository_trait_bounds() {
        fn check_send_sync<T: Send + Sync>() {}
        check_send_sync::<MockUserRepository>();
    }
}
