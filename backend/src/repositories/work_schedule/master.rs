use sqlx::PgPool;
use timekeeper_contract::work_schedules::{
    WorkScheduleDetailResponse, WorkScheduleListResponse, WorkScheduleResponse, WorkScheduleStatus,
};
use uuid::Uuid;

use super::{
    map_database_error,
    rows::{VersionSummaryRow, WorkScheduleRow},
    RepositoryResult, WorkScheduleRepositoryError,
};

const SCHEDULE_COLUMNS: &str =
    "id, code, name, description, status, created_by, created_at, updated_at";

#[derive(Debug, Clone)]
pub struct WorkScheduleListFilter {
    pub status: Option<WorkScheduleStatus>,
    pub query: Option<String>,
    pub page: i64,
    pub per_page: i64,
}

pub async fn list_work_schedules(
    pool: &PgPool,
    filter: &WorkScheduleListFilter,
) -> RepositoryResult<WorkScheduleListResponse> {
    let status = filter.status.map(|value| match value {
        WorkScheduleStatus::Active => "active",
        WorkScheduleStatus::Retired => "retired",
    });
    let query = filter.query.as_deref();
    let offset = (filter.page - 1) * filter.per_page;
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM work_schedules \
         WHERE ($1::TEXT IS NULL OR status = $1) \
         AND ($2::TEXT IS NULL OR code ILIKE '%' || $2 || '%' OR name ILIKE '%' || $2 || '%')",
    )
    .bind(status)
    .bind(query)
    .fetch_one(pool)
    .await?;
    let sql = format!(
        "SELECT {SCHEDULE_COLUMNS} FROM work_schedules \
         WHERE ($1::TEXT IS NULL OR status = $1) \
         AND ($2::TEXT IS NULL OR code ILIKE '%' || $2 || '%' OR name ILIKE '%' || $2 || '%') \
         ORDER BY updated_at DESC, id ASC LIMIT $3 OFFSET $4"
    );
    let rows = sqlx::query_as::<_, WorkScheduleRow>(&sql)
        .bind(status)
        .bind(query)
        .bind(filter.per_page)
        .bind(offset)
        .fetch_all(pool)
        .await?;
    let items = rows
        .into_iter()
        .map(WorkScheduleResponse::try_from)
        .collect::<RepositoryResult<Vec<_>>>()?;
    Ok(WorkScheduleListResponse {
        page: filter.page,
        per_page: filter.per_page,
        total,
        items,
    })
}

pub async fn create_work_schedule(
    pool: &PgPool,
    code: &str,
    name: &str,
    description: Option<&str>,
    created_by: &str,
) -> RepositoryResult<WorkScheduleResponse> {
    let id = Uuid::new_v4();
    let sql = format!(
        "INSERT INTO work_schedules (id, code, name, description, created_by) \
         VALUES ($1, $2, $3, $4, $5) RETURNING {SCHEDULE_COLUMNS}"
    );
    let row = sqlx::query_as::<_, WorkScheduleRow>(&sql)
        .bind(id)
        .bind(code)
        .bind(name)
        .bind(description)
        .bind(created_by)
        .fetch_one(pool)
        .await
        .map_err(map_database_error)?;
    WorkScheduleResponse::try_from(row)
}

pub async fn find_work_schedule(
    pool: &PgPool,
    id: Uuid,
) -> RepositoryResult<Option<WorkScheduleResponse>> {
    let sql = format!("SELECT {SCHEDULE_COLUMNS} FROM work_schedules WHERE id = $1");
    let row = sqlx::query_as::<_, WorkScheduleRow>(&sql)
        .bind(id)
        .fetch_optional(pool)
        .await?;
    row.map(WorkScheduleResponse::try_from).transpose()
}

pub async fn get_work_schedule_detail(
    pool: &PgPool,
    id: Uuid,
) -> RepositoryResult<WorkScheduleDetailResponse> {
    let schedule = find_work_schedule(pool, id)
        .await?
        .ok_or(WorkScheduleRepositoryError::NotFound)?;
    let rows = sqlx::query_as::<_, VersionSummaryRow>(
        "SELECT id, version_number, status, effective_from, effective_until, revision, \
         published_at FROM work_schedule_versions WHERE work_schedule_id = $1 \
         ORDER BY version_number DESC",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    let versions = rows
        .into_iter()
        .map(TryInto::try_into)
        .collect::<RepositoryResult<Vec<_>>>()?;
    Ok(WorkScheduleDetailResponse { schedule, versions })
}

pub async fn update_work_schedule(
    pool: &PgPool,
    id: Uuid,
    name: Option<&str>,
    description: Option<Option<&str>>,
) -> RepositoryResult<WorkScheduleResponse> {
    let sql = format!(
        "UPDATE work_schedules SET name = COALESCE($2, name), \
         description = CASE WHEN $3 THEN $4 ELSE description END, updated_at = NOW() \
         WHERE id = $1 RETURNING {SCHEDULE_COLUMNS}"
    );
    let row = sqlx::query_as::<_, WorkScheduleRow>(&sql)
        .bind(id)
        .bind(name)
        .bind(description.is_some())
        .bind(description.flatten())
        .fetch_optional(pool)
        .await?
        .ok_or(WorkScheduleRepositoryError::NotFound)?;
    WorkScheduleResponse::try_from(row)
}

pub async fn retire_work_schedule(
    pool: &PgPool,
    id: Uuid,
) -> RepositoryResult<WorkScheduleResponse> {
    let sql = format!(
        "UPDATE work_schedules SET status = 'retired', updated_at = NOW() \
         WHERE id = $1 RETURNING {SCHEDULE_COLUMNS}"
    );
    let row = sqlx::query_as::<_, WorkScheduleRow>(&sql)
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(WorkScheduleRepositoryError::NotFound)?;
    WorkScheduleResponse::try_from(row)
}
