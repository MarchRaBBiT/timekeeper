//! Shared repository utilities.

use sqlx::{Postgres, QueryBuilder};

/// Appends WHERE or AND to the query builder depending on whether a clause has already been added.
pub fn push_clause(builder: &mut QueryBuilder<'_, Postgres>, has_clause: &mut bool) {
    if *has_clause {
        builder.push(" AND ");
    } else {
        builder.push(" WHERE ");
        *has_clause = true;
    }
}
