//! Repository trait and common functionality
//!
//! This module defines the standard repository trait that all repository modules
//! should implement, along with transaction management utilities.

use crate::error::AppError;
use sqlx::PgPool;

/// Standard repository trait for database operations
///
/// All repository modules should implement this trait to ensure consistent
/// data access patterns and transaction handling.
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

/// Transaction management for database operations
///
/// Provides begin, commit, and rollback utilities for managing database
/// transactions across repository operations.
#[allow(dead_code)]
pub mod transaction {
    use crate::error::AppError;
    use sqlx::postgres::PgTransaction;
    use sqlx::PgPool;

    /// Begin a new database transaction
    ///
    /// Returns a transaction handle that can be used for multiple database operations.
    /// On success, the transaction can be committed via [`commit_transaction`].
    /// On failure, the transaction can be rolled back via [`rollback_transaction`].
    pub async fn begin_transaction(db: &PgPool) -> Result<PgTransaction<'_>, AppError> {
        db.begin()
            .await
            .map_err(|e| AppError::InternalServerError(e.into()))
    }

    /// Commit a transaction
    ///
    /// Commits all changes made within the transaction to the database.
    /// Returns error if commit fails.
    pub async fn commit_transaction(tx: PgTransaction<'_>) -> Result<(), AppError> {
        tx.commit()
            .await
            .map_err(|e| AppError::InternalServerError(e.into()))
    }

    /// Rollback a transaction
    ///
    /// Undoes all changes made within the transaction since it began.
    /// Returns error if rollback fails.
    pub async fn rollback_transaction(tx: PgTransaction<'_>) -> Result<(), AppError> {
        tx.rollback()
            .await
            .map_err(|e| AppError::InternalServerError(e.into()))
    }
}
