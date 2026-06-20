use std::str::FromStr;

use axum::{
    extract::{Extension, Path, Query, State},
    http::{header::LOCATION, HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    Json,
};
use timekeeper_contract::work_schedules::{
    AssignmentTarget, CreateWorkScheduleRequest, CreateWorkScheduleVersionRequest,
    ReplaceWorkScheduleVersionRequest, UpdateWorkScheduleRequest, WorkScheduleAssignmentListQuery,
    WorkScheduleAssignmentListResponse, WorkScheduleAssignmentRequest, WorkScheduleDetailResponse,
    WorkScheduleListQuery, WorkScheduleListResponse, WorkScheduleResponse,
    WorkScheduleVersionResponse,
};
use timekeeper_domain::work_schedules::{
    DayKind, PlannedBreak, PlannedWorkInterval, ScheduleDefinition, WeekdayRule,
};
use uuid::Uuid;
use validator::Validate;

use crate::{
    error::AppError,
    models::user::User,
    repositories::work_schedule::{
        self, AssignmentListFilter, WorkScheduleListFilter, WorkScheduleRepositoryError,
    },
    state::AppState,
    types::{DepartmentId, UserId},
};

const DEFAULT_PAGE: i64 = 1;
const DEFAULT_PER_PAGE: i64 = 25;
const MAX_PER_PAGE: i64 = 100;

pub async fn list_work_schedules(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(query): Query<WorkScheduleListQuery>,
) -> Result<Json<WorkScheduleListResponse>, AppError> {
    require_manager(&user)?;
    let normalized_query = normalize_optional(query.q);
    if normalized_query
        .as_ref()
        .is_some_and(|value| value.chars().count() > 100)
    {
        return Err(invalid_work_schedule("q must be at most 100 characters"));
    }
    let filter = WorkScheduleListFilter {
        status: query.status,
        query: normalized_query,
        page: normalize_page(query.page),
        per_page: normalize_per_page(query.per_page),
    };
    let response = work_schedule::list_work_schedules(state.read_pool(), &filter)
        .await
        .map_err(map_repository_error)?;
    Ok(Json(response))
}

pub async fn create_work_schedule(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<CreateWorkScheduleRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_system_admin(&user)?;
    payload.validate()?;
    let code = payload.code.trim();
    validate_code(code)?;
    let name = required_trimmed(&payload.name, "name")?;
    let description = normalize_optional(payload.description)
        .as_deref()
        .map(str::to_string);
    let response = work_schedule::create_work_schedule(
        &state.write_pool,
        code,
        name,
        description.as_deref(),
        &user.id.to_string(),
    )
    .await
    .map_err(map_repository_error)?;
    created_response(
        &format!("/api/admin/work-schedules/{}", response.id),
        response,
    )
}

pub async fn get_work_schedule(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> Result<Json<WorkScheduleDetailResponse>, AppError> {
    require_manager(&user)?;
    let id = parse_uuid(&id, "work_schedule_id")?;
    let response = work_schedule::get_work_schedule_detail(state.read_pool(), id)
        .await
        .map_err(map_repository_error)?;
    Ok(Json(response))
}

pub async fn update_work_schedule(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateWorkScheduleRequest>,
) -> Result<Json<WorkScheduleResponse>, AppError> {
    require_system_admin(&user)?;
    payload.validate()?;
    if payload.name.is_none() && payload.description.is_none() {
        return Err(invalid_work_schedule(
            "name or description must be provided",
        ));
    }
    let id = parse_uuid(&id, "work_schedule_id")?;
    let name = payload
        .name
        .as_deref()
        .map(|value| required_trimmed(value, "name"))
        .transpose()?;
    let description = payload.description.as_deref().map(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    });
    let response = work_schedule::update_work_schedule(&state.write_pool, id, name, description)
        .await
        .map_err(map_repository_error)?;
    Ok(Json(response))
}

pub async fn retire_work_schedule(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> Result<Json<WorkScheduleResponse>, AppError> {
    require_system_admin(&user)?;
    let id = parse_uuid(&id, "work_schedule_id")?;
    let response = work_schedule::retire_work_schedule(&state.write_pool, id)
        .await
        .map_err(map_repository_error)?;
    Ok(Json(response))
}

pub async fn create_work_schedule_version(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
    Json(payload): Json<CreateWorkScheduleVersionRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_system_admin(&user)?;
    validate_version_definition(
        payload.effective_from,
        payload.effective_until,
        &payload.timezone,
        payload.workday_boundary,
        payload.late_grace_minutes,
        payload.early_leave_grace_minutes,
        &payload.days,
    )?;
    let schedule_id = parse_uuid(&id, "work_schedule_id")?;
    let response = work_schedule::create_version(&state.write_pool, schedule_id, &payload)
        .await
        .map_err(map_repository_error)?;
    created_response(
        &format!(
            "/api/admin/work-schedules/{schedule_id}/versions/{}",
            response.id
        ),
        response,
    )
}

pub async fn get_work_schedule_version(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path((id, version_id)): Path<(String, String)>,
) -> Result<Json<WorkScheduleVersionResponse>, AppError> {
    require_manager(&user)?;
    let schedule_id = parse_uuid(&id, "work_schedule_id")?;
    let version_id = parse_uuid(&version_id, "version_id")?;
    let response = work_schedule::find_version(state.read_pool(), schedule_id, version_id)
        .await
        .map_err(map_repository_error)?;
    Ok(Json(response))
}

pub async fn replace_work_schedule_version(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path((id, version_id)): Path<(String, String)>,
    Json(payload): Json<ReplaceWorkScheduleVersionRequest>,
) -> Result<Json<WorkScheduleVersionResponse>, AppError> {
    require_system_admin(&user)?;
    if payload.revision < 1 {
        return Err(invalid_work_schedule("revision must be positive"));
    }
    validate_version_definition(
        payload.effective_from,
        payload.effective_until,
        &payload.timezone,
        payload.workday_boundary,
        payload.late_grace_minutes,
        payload.early_leave_grace_minutes,
        &payload.days,
    )?;
    let schedule_id = parse_uuid(&id, "work_schedule_id")?;
    let version_id = parse_uuid(&version_id, "version_id")?;
    let response =
        work_schedule::replace_version(&state.write_pool, schedule_id, version_id, &payload)
            .await
            .map_err(map_repository_error)?;
    Ok(Json(response))
}

pub async fn publish_work_schedule_version(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path((id, version_id)): Path<(String, String)>,
) -> Result<Json<WorkScheduleVersionResponse>, AppError> {
    require_system_admin(&user)?;
    let schedule_id = parse_uuid(&id, "work_schedule_id")?;
    let version_id = parse_uuid(&version_id, "version_id")?;
    let response = work_schedule::publish_version(
        &state.write_pool,
        schedule_id,
        version_id,
        &user.id.to_string(),
    )
    .await
    .map_err(map_repository_error)?;
    Ok(Json(response))
}

pub async fn delete_work_schedule_version(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path((id, version_id)): Path<(String, String)>,
) -> Result<StatusCode, AppError> {
    require_system_admin(&user)?;
    let schedule_id = parse_uuid(&id, "work_schedule_id")?;
    let version_id = parse_uuid(&version_id, "version_id")?;
    work_schedule::delete_version(&state.write_pool, schedule_id, version_id)
        .await
        .map_err(map_repository_error)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_work_schedule_assignments(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(query): Query<WorkScheduleAssignmentListQuery>,
) -> Result<Json<WorkScheduleAssignmentListResponse>, AppError> {
    require_manager(&user)?;
    let work_schedule_id = query
        .work_schedule_id
        .as_deref()
        .map(|value| parse_uuid(value, "work_schedule_id"))
        .transpose()?;
    let department_id = normalize_optional(query.department_id);
    if let Some(value) = department_id.as_deref() {
        DepartmentId::from_str(value)
            .map_err(|_| invalid_work_schedule("invalid department_id"))?;
    }
    let user_id = normalize_optional(query.user_id);
    if let Some(value) = user_id.as_deref() {
        UserId::from_str(value).map_err(|_| invalid_work_schedule("invalid user_id"))?;
    }
    let filter = AssignmentListFilter {
        work_schedule_id,
        department_id,
        user_id,
        page: normalize_page(query.page),
        per_page: normalize_per_page(query.per_page),
    };
    let response = work_schedule::list_assignments(state.read_pool(), &filter)
        .await
        .map_err(map_repository_error)?;
    Ok(Json(response))
}

pub async fn create_work_schedule_assignment(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<WorkScheduleAssignmentRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_system_admin(&user)?;
    validate_assignment(&payload)?;
    let schedule_id = parse_uuid(&payload.work_schedule_id, "work_schedule_id")?;
    let response = work_schedule::create_assignment(
        &state.write_pool,
        &payload,
        schedule_id,
        &user.id.to_string(),
    )
    .await
    .map_err(map_repository_error)?;
    created_response(
        &format!("/api/admin/work-schedule-assignments/{}", response.id),
        response,
    )
}

pub async fn delete_work_schedule_assignment(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    require_system_admin(&user)?;
    let id = parse_uuid(&id, "assignment_id")?;
    work_schedule::delete_assignment(&state.write_pool, id)
        .await
        .map_err(map_repository_error)?;
    Ok(StatusCode::NO_CONTENT)
}

fn validate_version_definition(
    effective_from: chrono::NaiveDate,
    effective_until: Option<chrono::NaiveDate>,
    timezone: &str,
    workday_boundary: chrono::NaiveTime,
    late_grace_minutes: i32,
    early_leave_grace_minutes: i32,
    days: &[timekeeper_contract::work_schedules::WeekdayRuleInput],
) -> Result<(), AppError> {
    if timezone != timezone.trim() {
        return Err(invalid_work_schedule(
            "timezone must not contain surrounding whitespace",
        ));
    }
    if !(0..=1440).contains(&late_grace_minutes) || !(0..=1440).contains(&early_leave_grace_minutes)
    {
        return Err(invalid_work_schedule(
            "grace minutes must be between 0 and 1440",
        ));
    }
    let definition = ScheduleDefinition {
        effective_from,
        effective_until,
        timezone: timezone.trim().to_string(),
        workday_boundary,
        days: days
            .iter()
            .map(|day| WeekdayRule {
                weekday: day.weekday,
                day_kind: match day.day_kind {
                    timekeeper_contract::work_schedules::DayKind::WorkingDay => DayKind::WorkingDay,
                    timekeeper_contract::work_schedules::DayKind::NonWorkingDay => {
                        DayKind::NonWorkingDay
                    }
                },
                work_intervals: day
                    .work_intervals
                    .iter()
                    .map(|interval| PlannedWorkInterval {
                        start_time: interval.start_time,
                        start_day_offset: interval.start_day_offset,
                        end_time: interval.end_time,
                        end_day_offset: interval.end_day_offset,
                    })
                    .collect(),
                planned_breaks: day
                    .planned_breaks
                    .iter()
                    .map(|planned_break| PlannedBreak {
                        start_time: planned_break.start_time,
                        start_day_offset: planned_break.start_day_offset,
                        end_time: planned_break.end_time,
                        end_day_offset: planned_break.end_day_offset,
                    })
                    .collect(),
            })
            .collect(),
    };
    definition
        .validate()
        .map_err(|error| AppError::UnprocessableEntityWithCode {
            message: error.to_string(),
            code: "INVALID_SCHEDULE_INTERVALS".to_string(),
        })
}

fn validate_assignment(payload: &WorkScheduleAssignmentRequest) -> Result<(), AppError> {
    if payload
        .valid_until
        .is_some_and(|until| until <= payload.valid_from)
    {
        return Err(invalid_work_schedule(
            "valid_until must be later than valid_from",
        ));
    }
    match &payload.target {
        AssignmentTarget::Organization => Ok(()),
        AssignmentTarget::Department { department_id } => DepartmentId::from_str(department_id)
            .map(|_| ())
            .map_err(|_| invalid_work_schedule("invalid department_id")),
        AssignmentTarget::User { user_id } => UserId::from_str(user_id)
            .map(|_| ())
            .map_err(|_| invalid_work_schedule("invalid user_id")),
    }
}

fn map_repository_error(error: WorkScheduleRepositoryError) -> AppError {
    match error {
        WorkScheduleRepositoryError::NotFound => {
            AppError::NotFound("Work schedule resource not found".to_string())
        }
        WorkScheduleRepositoryError::CodeConflict => AppError::ConflictWithCode {
            message: "Work schedule code already exists".to_string(),
            code: "WORK_SCHEDULE_CODE_CONFLICT".to_string(),
        },
        WorkScheduleRepositoryError::PeriodOverlap => AppError::ConflictWithCode {
            message: "Effective period overlaps an existing record".to_string(),
            code: "EFFECTIVE_PERIOD_OVERLAP".to_string(),
        },
        WorkScheduleRepositoryError::PublishedVersionImmutable => AppError::ConflictWithCode {
            message: "Published work schedule version is immutable".to_string(),
            code: "PUBLISHED_VERSION_IMMUTABLE".to_string(),
        },
        WorkScheduleRepositoryError::RevisionConflict => AppError::ConflictWithCode {
            message: "Draft revision does not match".to_string(),
            code: "REVISION_CONFLICT".to_string(),
        },
        WorkScheduleRepositoryError::RetiredSchedule => AppError::ConflictWithCode {
            message: "Retired work schedule cannot be changed or assigned".to_string(),
            code: "WORK_SCHEDULE_RETIRED".to_string(),
        },
        WorkScheduleRepositoryError::InvalidReference => AppError::BadRequestWithCode {
            message: "Referenced user or department does not exist".to_string(),
            code: "INVALID_WORK_SCHEDULE_REFERENCE".to_string(),
        },
        WorkScheduleRepositoryError::CorruptData(message) => {
            AppError::InternalServerError(anyhow::anyhow!(message))
        }
        WorkScheduleRepositoryError::Sqlx(error) => AppError::InternalServerError(error.into()),
    }
}

fn require_manager(user: &User) -> Result<(), AppError> {
    if user.is_manager() || user.is_system_admin() {
        Ok(())
    } else {
        Err(AppError::Forbidden("Forbidden".to_string()))
    }
}

fn require_system_admin(user: &User) -> Result<(), AppError> {
    if user.is_system_admin() {
        Ok(())
    } else {
        Err(AppError::Forbidden("Forbidden".to_string()))
    }
}

fn parse_uuid(value: &str, field: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value).map_err(|_| invalid_work_schedule(&format!("invalid {field}")))
}

fn validate_code(code: &str) -> Result<(), AppError> {
    if code.is_empty()
        || !code
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(invalid_work_schedule(
            "code must contain only ASCII letters, numbers, hyphens, or underscores",
        ));
    }
    Ok(())
}

fn required_trimmed<'a>(value: &'a str, field: &str) -> Result<&'a str, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(invalid_work_schedule(&format!("{field} is required")));
    }
    Ok(trimmed)
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn normalize_page(value: Option<i64>) -> i64 {
    value.unwrap_or(DEFAULT_PAGE).clamp(1, 1_000_000)
}

fn normalize_per_page(value: Option<i64>) -> i64 {
    value.unwrap_or(DEFAULT_PER_PAGE).clamp(1, MAX_PER_PAGE)
}

fn invalid_work_schedule(message: &str) -> AppError {
    AppError::BadRequestWithCode {
        message: message.to_string(),
        code: "INVALID_WORK_SCHEDULE".to_string(),
    }
}

fn created_response<T>(location: &str, body: T) -> Result<impl IntoResponse, AppError>
where
    T: serde::Serialize,
{
    let mut headers = HeaderMap::new();
    let location = HeaderValue::from_str(location)
        .map_err(|error| AppError::InternalServerError(error.into()))?;
    headers.insert(LOCATION, location);
    Ok((StatusCode::CREATED, headers, Json(body)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_validation_accepts_safe_identifiers() {
        assert!(validate_code("standard-2026_01").is_ok());
        assert!(validate_code("bad code").is_err());
        assert!(validate_code("../bad").is_err());
    }

    #[test]
    fn pagination_is_bounded() {
        assert_eq!(normalize_page(Some(0)), 1);
        assert_eq!(normalize_per_page(Some(500)), 100);
    }
}
