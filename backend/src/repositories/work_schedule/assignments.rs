use sqlx::PgPool;
use timekeeper_contract::work_schedules::{
    AssignmentTarget, WorkScheduleAssignmentListResponse, WorkScheduleAssignmentRequest,
    WorkScheduleAssignmentResponse,
};
use uuid::Uuid;

use super::{
    map_database_error, rows::AssignmentRow, RepositoryResult, WorkScheduleRepositoryError,
};

const ASSIGNMENT_COLUMNS: &str = "id, work_schedule_id, user_id, department_id, \
    is_org_default, valid_from, valid_until, created_by, created_at";

#[derive(Debug, Clone)]
pub struct AssignmentListFilter {
    pub work_schedule_id: Option<Uuid>,
    pub department_id: Option<String>,
    pub user_id: Option<String>,
    pub page: i64,
    pub per_page: i64,
}

pub async fn list_assignments(
    pool: &PgPool,
    filter: &AssignmentListFilter,
) -> RepositoryResult<WorkScheduleAssignmentListResponse> {
    let offset = (filter.page - 1) * filter.per_page;
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM work_schedule_assignments \
         WHERE ($1::UUID IS NULL OR work_schedule_id = $1) \
         AND ($2::TEXT IS NULL OR department_id = $2) \
         AND ($3::TEXT IS NULL OR user_id = $3)",
    )
    .bind(filter.work_schedule_id)
    .bind(filter.department_id.as_deref())
    .bind(filter.user_id.as_deref())
    .fetch_one(pool)
    .await?;
    let sql = format!(
        "SELECT {ASSIGNMENT_COLUMNS} FROM work_schedule_assignments \
         WHERE ($1::UUID IS NULL OR work_schedule_id = $1) \
         AND ($2::TEXT IS NULL OR department_id = $2) \
         AND ($3::TEXT IS NULL OR user_id = $3) \
         ORDER BY valid_from DESC, id ASC LIMIT $4 OFFSET $5"
    );
    let rows = sqlx::query_as::<_, AssignmentRow>(&sql)
        .bind(filter.work_schedule_id)
        .bind(filter.department_id.as_deref())
        .bind(filter.user_id.as_deref())
        .bind(filter.per_page)
        .bind(offset)
        .fetch_all(pool)
        .await?;
    let items = rows
        .into_iter()
        .map(WorkScheduleAssignmentResponse::try_from)
        .collect::<RepositoryResult<Vec<_>>>()?;
    Ok(WorkScheduleAssignmentListResponse {
        page: filter.page,
        per_page: filter.per_page,
        total,
        items,
    })
}

pub async fn create_assignment(
    pool: &PgPool,
    request: &WorkScheduleAssignmentRequest,
    schedule_id: Uuid,
    created_by: &str,
) -> RepositoryResult<WorkScheduleAssignmentResponse> {
    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM work_schedules WHERE id = $1")
            .bind(schedule_id)
            .fetch_optional(pool)
            .await?;
    match status.as_deref() {
        None => return Err(WorkScheduleRepositoryError::NotFound),
        Some("retired") => return Err(WorkScheduleRepositoryError::RetiredSchedule),
        Some(_) => {}
    }
    let (user_id, department_id, is_org_default) = match &request.target {
        AssignmentTarget::Organization => (None, None, true),
        AssignmentTarget::Department { department_id } => {
            (None, Some(department_id.as_str()), false)
        }
        AssignmentTarget::User { user_id } => (Some(user_id.as_str()), None, false),
    };
    let id = Uuid::new_v4();
    let sql = format!(
        "INSERT INTO work_schedule_assignments \
         (id, work_schedule_id, user_id, department_id, is_org_default, \
          valid_from, valid_until, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING {ASSIGNMENT_COLUMNS}"
    );
    let row = sqlx::query_as::<_, AssignmentRow>(&sql)
        .bind(id)
        .bind(schedule_id)
        .bind(user_id)
        .bind(department_id)
        .bind(is_org_default)
        .bind(request.valid_from)
        .bind(request.valid_until)
        .bind(created_by)
        .fetch_one(pool)
        .await
        .map_err(map_database_error)?;
    WorkScheduleAssignmentResponse::try_from(row)
}

pub async fn delete_assignment(pool: &PgPool, id: Uuid) -> RepositoryResult<()> {
    let result = sqlx::query("DELETE FROM work_schedule_assignments WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(WorkScheduleRepositoryError::NotFound);
    }
    Ok(())
}
