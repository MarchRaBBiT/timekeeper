//! Transaction management utilities for repositories.

use crate::error::AppError;
use sqlx::postgres::PgTransaction;
use sqlx::PgPool;

/// Begin a new database transaction.
///
/// Returns a transaction handle that can be used for multiple database operations.
/// On success, the transaction can be committed via [`commit_transaction`].
/// On failure, the transaction can be rolled back via [`rollback_transaction`].
pub async fn begin_transaction(db: &PgPool) -> Result<PgTransaction<'_>, AppError> {
    db.begin()
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))
}

/// Commit a transaction.
///
/// Commits all changes made within the transaction to the database.
pub async fn commit_transaction(tx: PgTransaction<'_>) -> Result<(), AppError> {
    tx.commit()
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))
}

/// Rollback a transaction.
///
/// Undoes all changes made within the transaction since it began.
#[allow(dead_code)]
pub async fn rollback_transaction(tx: PgTransaction<'_>) -> Result<(), AppError> {
    tx.rollback()
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transaction_module_is_compilable() {
        let _ = begin_transaction;
        let _ = commit_transaction;
        let _ = rollback_transaction;
    }
}
