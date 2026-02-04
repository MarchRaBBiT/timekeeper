use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, QueryBuilder};

use crate::models::{
    request::RequestStatus,
    subject_request::{DataSubjectRequest, DataSubjectRequestType},
};

#[derive(Debug, Clone, Default)]
pub struct SubjectRequestFilters {
    pub status: Option<RequestStatus>,
    pub request_type: Option<DataSubjectRequestType>,
    pub user_id: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

pub async fn insert_subject_request(
    pool: &PgPool,
    request: &DataSubjectRequest,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO subject_requests \
         (id, user_id, request_type, status, details, approved_by, approved_at, rejected_by, rejected_at, \
          cancelled_at, decision_comment, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
    )
    .bind(&request.id)
    .bind(&request.user_id)
    .bind(request.request_type.db_value())
    .bind(request.status.db_value())
    .bind(&request.details)
    .bind(&request.approved_by)
    .bind(request.approved_at)
    .bind(&request.rejected_by)
    .bind(request.rejected_at)
    .bind(request.cancelled_at)
    .bind(&request.decision_comment)
    .bind(request.created_at)
    .bind(request.updated_at)
    .execute(pool)
    .await
    .map(|_| ())
}

pub async fn list_subject_requests_by_user(
    pool: &PgPool,
    user_id: &str,
) -> Result<Vec<DataSubjectRequest>, sqlx::Error> {
    sqlx::query_as::<_, DataSubjectRequest>(
        "SELECT id, user_id, request_type, status, details, approved_by, approved_at, rejected_by, \
         rejected_at, cancelled_at, decision_comment, created_at, updated_at \
         FROM subject_requests WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn fetch_subject_request(
    pool: &PgPool,
    id: &str,
) -> Result<Option<DataSubjectRequest>, sqlx::Error> {
    sqlx::query_as::<_, DataSubjectRequest>(
        "SELECT id, user_id, request_type, status, details, approved_by, approved_at, rejected_by, \
         rejected_at, cancelled_at, decision_comment, created_at, updated_at \
         FROM subject_requests WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn cancel_subject_request(
    pool: &PgPool,
    id: &str,
    user_id: &str,
    cancelled_at: DateTime<Utc>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE subject_requests SET status = $1, cancelled_at = $2, updated_at = $2 \
         WHERE id = $3 AND user_id = $4 AND status = 'pending'",
    )
    .bind(RequestStatus::Cancelled.db_value())
    .bind(cancelled_at)
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn approve_subject_request(
    pool: &PgPool,
    id: &str,
    approver_id: &str,
    comment: &str,
    approved_at: DateTime<Utc>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE subject_requests SET status = $1, approved_by = $2, approved_at = $3, \
         decision_comment = $4, updated_at = $3 \
         WHERE id = $5 AND status = 'pending'",
    )
    .bind(RequestStatus::Approved.db_value())
    .bind(approver_id)
    .bind(approved_at)
    .bind(comment)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn reject_subject_request(
    pool: &PgPool,
    id: &str,
    approver_id: &str,
    comment: &str,
    rejected_at: DateTime<Utc>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE subject_requests SET status = $1, rejected_by = $2, rejected_at = $3, \
         decision_comment = $4, updated_at = $3 \
         WHERE id = $5 AND status = 'pending'",
    )
    .bind(RequestStatus::Rejected.db_value())
    .bind(approver_id)
    .bind(rejected_at)
    .bind(comment)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn list_subject_requests(
    pool: &PgPool,
    filters: &SubjectRequestFilters,
    per_page: i64,
    offset: i64,
) -> Result<(Vec<DataSubjectRequest>, i64), sqlx::Error> {
    let items = query_subject_requests(pool, filters, Some((per_page, offset))).await?;

    let mut count_builder: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM subject_requests");
    let mut has_clause = false;
    apply_subject_request_filters(&mut count_builder, &mut has_clause, filters);
    let total = count_builder
        .build_query_scalar::<i64>()
        .fetch_one(pool)
        .await?;

    Ok((items, total))
}

async fn query_subject_requests(
    pool: &PgPool,
    filters: &SubjectRequestFilters,
    pagination: Option<(i64, i64)>,
) -> Result<Vec<DataSubjectRequest>, sqlx::Error> {
    let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT id, user_id, request_type, status, details, approved_by, approved_at, rejected_by, \
         rejected_at, cancelled_at, decision_comment, created_at, updated_at FROM subject_requests",
    );
    let mut has_clause = false;
    apply_subject_request_filters(&mut builder, &mut has_clause, filters);
    builder.push(" ORDER BY created_at DESC, id DESC");

    if let Some((per_page, offset)) = pagination {
        builder
            .push(" LIMIT ")
            .push_bind(per_page)
            .push(" OFFSET ")
            .push_bind(offset);
    }

    builder
        .build_query_as::<DataSubjectRequest>()
        .fetch_all(pool)
        .await
}

fn apply_subject_request_filters<'a>(
    builder: &mut QueryBuilder<'a, Postgres>,
    has_clause: &mut bool,
    filters: &SubjectRequestFilters,
) {
    if let Some(status) = filters.status.as_ref() {
        push_clause(builder, has_clause);
        builder
            .push("status = ")
            .push_bind(status.db_value().to_string());
    }
    if let Some(request_type) = filters.request_type.as_ref() {
        push_clause(builder, has_clause);
        builder
            .push("request_type = ")
            .push_bind(request_type.db_value().to_string());
    }
    if let Some(user_id) = filters.user_id.as_ref() {
        push_clause(builder, has_clause);
        builder.push("user_id = ").push_bind(user_id.to_string());
    }
    if let Some(from) = filters.from.as_ref() {
        push_clause(builder, has_clause);
        builder.push("created_at >= ").push_bind(from.to_owned());
    }
    if let Some(to) = filters.to.as_ref() {
        push_clause(builder, has_clause);
        builder.push("created_at <= ").push_bind(to.to_owned());
    }
}

fn push_clause<'a>(builder: &mut QueryBuilder<'a, Postgres>, has_clause: &mut bool) {
    if *has_clause {
        builder.push(" AND ");
    } else {
        builder.push(" WHERE ");
        *has_clause = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subject_request_filters_default() {
        let filters = SubjectRequestFilters::default();
        assert!(filters.status.is_none());
        assert!(filters.request_type.is_none());
        assert!(filters.user_id.is_none());
    }
}
