//! Repository trait and common functionality
//!
//! This module defines the standard repository trait that all repository modules
//! should implement.

use crate::error::AppError;
use sqlx::PgPool;

/// Standard repository trait for database operations
///
/// All repository modules should implement this trait to ensure consistent
/// data access patterns.
#[allow(async_fn_in_trait, dead_code)]
pub trait Repository<T> {
    /// Target table name.
    const TABLE: &'static str;
    /// Primary key type for the record.
    type Id;
    /// Find all records of type T
    async fn find_all(&self, db: &PgPool) -> Result<Vec<T>, AppError>;

    /// Find a single record by ID
    async fn find_by_id(&self, db: &PgPool, id: Self::Id) -> Result<T, AppError>;

    /// Create a new record
    async fn create(&self, db: &PgPool, item: &T) -> Result<T, AppError>;

    /// Update an existing record
    async fn update(&self, db: &PgPool, item: &T) -> Result<T, AppError>;

    /// Delete a record by ID
    async fn delete(&self, db: &PgPool, id: Self::Id) -> Result<(), AppError>;
}
