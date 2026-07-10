use std::str::FromStr;

use axum::{
    extract::{Extension, Path, Query, State},
    http::{header::LOCATION, HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono::{NaiveDate, Utc};
use timekeeper_app::user_workdays::{ListUserWorkdays, ListUserWorkdaysCommand};
use timekeeper_app::work_schedules::{ResolveWorkday, ResolveWorkdayCommand, ResolveWorkdayError};
use timekeeper_contract::work_schedules::{
    AssignmentTarget, BulkWorkScheduleAssignmentFailure, BulkWorkScheduleAssignmentRequest,
    BulkWorkScheduleAssignmentResponse, CloseWorkScheduleMonthRequest,
    CloseWorkScheduleMonthResponse, CreateWorkScheduleRequest, CreateWorkScheduleVersionRequest,
    FlexPolicyInput, GenerateWorkScheduleProjectionsRequest,
    GenerateWorkScheduleProjectionsResponse, MonthlyClosingStatus, MonthlyClosingTransitionRequest,
    MonthlyClosingWorkflowResponse, OvertimeMonitorQuery, OvertimeMonitorResponse,
    OvertimeMonitorSettingsRequest, OvertimeMonitorSettingsResponse,
    ReplaceWorkScheduleVersionRequest, UpdateWorkScheduleRequest, WorkScheduleAnomalyListQuery,
    WorkScheduleAnomalyListResponse, WorkScheduleAssignmentListQuery,
    WorkScheduleAssignmentListResponse, WorkScheduleAssignmentRequest,
    WorkScheduleCalendarDayResponse, WorkScheduleCalendarResponse, WorkScheduleDetailResponse,
    WorkScheduleListQuery, WorkScheduleListResponse, WorkScheduleProjectionError,
    WorkScheduleResponse, WorkScheduleType, WorkScheduleVersionResponse,
};
use timekeeper_domain::work_schedules::{
    CoreTimeWindow, DayKind, FlexPolicy, PlannedBreak, PlannedWorkInterval, ScheduleDefinition,
    ScheduleType, SettlementPeriod, SettlementPeriodUnit, WeekdayRule,
};
use timekeeper_infra_postgres::work_schedules::WorkdayResolverPostgresRepository;
use uuid::Uuid;
use validator::Validate;

use crate::{
    error::AppError,
    handlers::work_schedules::resolved_workday_to_response,
    models::user::User,
    repositories::department::{can_manager_approve, list_subordinate_user_ids},
    repositories::work_schedule::{
        self, AssignmentListFilter, WorkScheduleListFilter, WorkScheduleRepositoryError,
    },
    state::AppState,
    types::{DepartmentId, UserId},
};

const DEFAULT_PAGE: i64 = 1;
const DEFAULT_PER_PAGE: i64 = 25;
const MAX_PER_PAGE: i64 = 100;
const MAX_PROJECTION_USERS: usize = 500;
const MAX_BULK_ASSIGNMENT_TARGETS: usize = 500;

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
    validate_version_definition(VersionDefinitionInput {
        effective_from: payload.effective_from,
        effective_until: payload.effective_until,
        timezone: &payload.timezone,
        workday_boundary: payload.workday_boundary,
        late_grace_minutes: payload.late_grace_minutes,
        early_leave_grace_minutes: payload.early_leave_grace_minutes,
        schedule_type: payload.schedule_type,
        flex_policy: payload.flex_policy.as_ref(),
        days: &payload.days,
    })?;
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
    validate_version_definition(VersionDefinitionInput {
        effective_from: payload.effective_from,
        effective_until: payload.effective_until,
        timezone: &payload.timezone,
        workday_boundary: payload.workday_boundary,
        late_grace_minutes: payload.late_grace_minutes,
        early_leave_grace_minutes: payload.early_leave_grace_minutes,
        schedule_type: payload.schedule_type,
        flex_policy: payload.flex_policy.as_ref(),
        days: &payload.days,
    })?;
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

pub async fn generate_work_schedule_projections(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<GenerateWorkScheduleProjectionsRequest>,
) -> Result<Json<GenerateWorkScheduleProjectionsResponse>, AppError> {
    require_system_admin(&user)?;
    validate_range(payload.from, payload.to)?;
    if payload.user_ids.is_empty() {
        return Err(invalid_work_schedule("user_ids must not be empty"));
    }
    if payload.user_ids.len() > MAX_PROJECTION_USERS {
        return Err(invalid_work_schedule(
            "user_ids exceeds the supported maximum",
        ));
    }
    for user_id in &payload.user_ids {
        UserId::from_str(user_id).map_err(|_| invalid_work_schedule("invalid user_id"))?;
    }

    let repository = WorkdayResolverPostgresRepository::new(state.write_pool.clone());
    let resolver = ResolveWorkday::new(repository.clone(), repository.clone(), repository);
    let mut projected = 0_usize;
    let mut already_locked = 0_usize;
    let mut not_configured = 0_usize;
    let mut errors = Vec::new();
    let resolved_at = Utc::now();

    for user_id in &payload.user_ids {
        for work_date in dates_inclusive(payload.from, payload.to)? {
            match resolver
                .execute(ResolveWorkdayCommand {
                    user_id: user_id.clone(),
                    work_date,
                    resolved_at,
                })
                .await
            {
                Ok(workday) => {
                    projected += 1;
                    if workday.locked_at.is_some() {
                        already_locked += 1;
                    }
                }
                Err(ResolveWorkdayError::WorkScheduleNotConfigured) => {
                    not_configured += 1;
                    errors.push(WorkScheduleProjectionError {
                        user_id: user_id.clone(),
                        work_date,
                        code: "WORK_SCHEDULE_NOT_CONFIGURED".to_string(),
                        message: "work schedule is not configured".to_string(),
                    });
                }
                Err(error) => {
                    errors.push(projection_error(user_id, work_date, error));
                }
            }
        }
    }

    Ok(Json(GenerateWorkScheduleProjectionsResponse {
        from: payload.from,
        to: payload.to,
        requested_users: payload.user_ids.len(),
        projected,
        already_locked,
        not_configured,
        errors,
    }))
}

pub async fn get_work_schedule_calendar(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(user_id): Path<String>,
    Query(query): Query<WorkScheduleAnomalyListQuery>,
) -> Result<Json<WorkScheduleCalendarResponse>, AppError> {
    validate_range(query.from, query.to)?;
    let target =
        UserId::from_str(&user_id).map_err(|_| invalid_work_schedule("invalid user_id"))?;
    authorize_scope(&state, &user, target).await?;

    let repository = WorkdayResolverPostgresRepository::new(state.read_pool().clone());
    let use_case = ListUserWorkdays::new(repository);
    let workdays = use_case
        .execute(ListUserWorkdaysCommand {
            user_id: user_id.clone(),
            from: query.from,
            to: query.to,
        })
        .await
        .map_err(|error| AppError::InternalServerError(anyhow::anyhow!(error.to_string())))?;
    let mut workdays_by_date = workdays
        .into_iter()
        .map(|workday| (workday.work_date, resolved_workday_to_response(workday)))
        .collect::<std::collections::HashMap<_, _>>();
    let mut attendance_by_date = work_schedule::list_user_attendance_calendar(
        state.read_pool(),
        &user_id,
        query.from,
        query.to,
    )
    .await
    .map_err(map_repository_error)?;
    let mut leave_by_date =
        work_schedule::list_user_leave_calendar(state.read_pool(), &user_id, query.from, query.to)
            .await
            .map_err(map_repository_error)?;
    let anomalies = work_schedule::list_anomalies(
        state.read_pool(),
        Some(vec![user_id.clone()]),
        query.from,
        query.to,
    )
    .await
    .map_err(map_repository_error)?;
    let mut anomalies_by_date: std::collections::HashMap<_, Vec<_>> =
        std::collections::HashMap::new();
    for anomaly in anomalies {
        anomalies_by_date
            .entry(anomaly.work_date)
            .or_default()
            .push(anomaly);
    }

    let mut days = Vec::new();
    for work_date in dates_inclusive(query.from, query.to)? {
        days.push(WorkScheduleCalendarDayResponse {
            work_date,
            resolved_workday: workdays_by_date.remove(&work_date),
            attendance: attendance_by_date.remove(&work_date),
            anomalies: anomalies_by_date.remove(&work_date).unwrap_or_default(),
            leave: leave_by_date.remove(&work_date),
        });
    }

    Ok(Json(WorkScheduleCalendarResponse {
        user_id,
        from: query.from,
        to: query.to,
        days,
    }))
}

pub async fn list_work_schedule_anomalies(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(query): Query<WorkScheduleAnomalyListQuery>,
) -> Result<Json<WorkScheduleAnomalyListResponse>, AppError> {
    require_manager(&user)?;
    validate_range(query.from, query.to)?;
    let user_ids = if let Some(user_id) = query.user_id {
        let target =
            UserId::from_str(&user_id).map_err(|_| invalid_work_schedule("invalid user_id"))?;
        authorize_scope(&state, &user, target).await?;
        Some(vec![user_id])
    } else if user.is_system_admin() {
        None
    } else {
        Some(
            list_subordinate_user_ids(state.read_pool(), user.id)
                .await
                .map_err(|error| AppError::InternalServerError(error.into()))?,
        )
    };
    let items = work_schedule::list_anomalies(state.read_pool(), user_ids, query.from, query.to)
        .await
        .map_err(map_repository_error)?;
    Ok(Json(WorkScheduleAnomalyListResponse {
        from: query.from,
        to: query.to,
        items,
    }))
}

pub async fn list_overtime_monitor(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(query): Query<OvertimeMonitorQuery>,
) -> Result<Json<OvertimeMonitorResponse>, AppError> {
    require_manager(&user)?;
    if !(1..=12).contains(&query.month) {
        return Err(invalid_work_schedule("month must be between 1 and 12"));
    }
    if !(1900..=9999).contains(&query.year) {
        return Err(invalid_work_schedule("year must be between 1900 and 9999"));
    }
    let user_ids = if user.is_system_admin() {
        None
    } else {
        Some(
            list_subordinate_user_ids(state.read_pool(), user.id)
                .await
                .map_err(|error| AppError::InternalServerError(error.into()))?,
        )
    };
    let scoped_user_ids = match user_ids {
        Some(ids) => ids,
        None => sqlx::query_scalar::<_, String>("SELECT id FROM users ORDER BY id")
            .fetch_all(state.read_pool())
            .await
            .map_err(|error| AppError::InternalServerError(error.into()))?,
    };
    let response = work_schedule::list_overtime_monitor(
        state.read_pool(),
        &scoped_user_ids,
        query.year,
        query.month,
    )
    .await
    .map_err(map_repository_error)?;
    Ok(Json(response))
}

pub async fn get_overtime_monitor_settings(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<OvertimeMonitorSettingsResponse>, AppError> {
    require_system_admin(&user)?;
    let response =
        work_schedule::get_overtime_monitor_settings(state.read_pool(), Utc::now().date_naive())
            .await
            .map_err(map_repository_error)?;
    Ok(Json(response))
}

pub async fn upsert_overtime_monitor_settings(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<OvertimeMonitorSettingsRequest>,
) -> Result<Json<OvertimeMonitorSettingsResponse>, AppError> {
    require_system_admin(&user)?;
    validate_overtime_monitor_settings(&payload)?;
    let response = work_schedule::upsert_overtime_monitor_settings(&state.write_pool, &payload)
        .await
        .map_err(map_repository_error)?;
    Ok(Json(response))
}

pub async fn bulk_create_work_schedule_assignments(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<BulkWorkScheduleAssignmentRequest>,
) -> Result<Json<BulkWorkScheduleAssignmentResponse>, AppError> {
    require_system_admin(&user)?;
    if payload.targets.is_empty() {
        return Err(invalid_work_schedule("targets must not be empty"));
    }
    if payload.targets.len() > MAX_BULK_ASSIGNMENT_TARGETS {
        return Err(invalid_work_schedule(
            "targets exceeds the supported maximum",
        ));
    }
    if payload
        .valid_until
        .is_some_and(|until| until <= payload.valid_from)
    {
        return Err(invalid_work_schedule(
            "valid_until must be later than valid_from",
        ));
    }
    let schedule_id = parse_uuid(&payload.work_schedule_id, "work_schedule_id")?;
    let mut created = Vec::new();
    let mut failed = Vec::new();

    for target in payload.targets {
        let request = WorkScheduleAssignmentRequest {
            work_schedule_id: payload.work_schedule_id.clone(),
            target: target.clone(),
            valid_from: payload.valid_from,
            valid_until: payload.valid_until,
        };
        if let Err(error) = validate_assignment(&request) {
            failed.push(failure(
                target,
                "INVALID_WORK_SCHEDULE",
                format!("{error:?}"),
            ));
            continue;
        }
        match work_schedule::create_assignment(
            &state.write_pool,
            &request,
            schedule_id,
            &user.id.to_string(),
        )
        .await
        {
            Ok(response) => created.push(response),
            Err(error) => {
                let (code, message) = repository_error_code_message(&error);
                failed.push(failure(target, code, message));
            }
        }
    }

    Ok(Json(BulkWorkScheduleAssignmentResponse { created, failed }))
}

pub async fn close_work_schedule_month(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<CloseWorkScheduleMonthRequest>,
) -> Result<Json<CloseWorkScheduleMonthResponse>, AppError> {
    require_system_admin(&user)?;
    if !(1..=12).contains(&payload.month) {
        return Err(invalid_work_schedule("month must be between 1 and 12"));
    }
    if !(1900..=9999).contains(&payload.year) {
        return Err(invalid_work_schedule("year must be between 1900 and 9999"));
    }
    for user_id in &payload.user_ids {
        UserId::from_str(user_id).map_err(|_| invalid_work_schedule("invalid user_id"))?;
    }
    let reason = payload
        .reason
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty());
    if reason.is_some_and(|value| value.chars().count() > 500) {
        return Err(invalid_work_schedule(
            "reason must be at most 500 characters",
        ));
    }
    let (from, to, locked_count) = work_schedule::close_month(
        &state.write_pool,
        payload.year,
        payload.month,
        &payload.user_ids,
        &user.id.to_string(),
        reason,
    )
    .await
    .map_err(map_repository_error)?;
    Ok(Json(CloseWorkScheduleMonthResponse {
        year: payload.year,
        month: payload.month,
        from,
        to,
        locked_count,
    }))
}

pub async fn self_confirm_monthly_closing(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<MonthlyClosingTransitionRequest>,
) -> Result<Json<MonthlyClosingWorkflowResponse>, AppError> {
    validate_monthly_transition_payload(&payload)?;
    let reason = normalized_reason(payload.reason.as_deref())?;
    let response = work_schedule::transition_monthly_closing(
        &state.write_pool,
        &user.id.to_string(),
        payload.year,
        payload.month,
        MonthlyClosingStatus::SelfConfirmed,
        &user.id.to_string(),
        reason,
    )
    .await
    .map_err(map_repository_error)?;
    Ok(Json(response))
}

pub async fn approve_monthly_closing(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(user_id): Path<String>,
    Json(payload): Json<MonthlyClosingTransitionRequest>,
) -> Result<Json<MonthlyClosingWorkflowResponse>, AppError> {
    validate_monthly_transition_payload(&payload)?;
    let target =
        UserId::from_str(&user_id).map_err(|_| invalid_work_schedule("invalid user_id"))?;
    if target == user.id {
        return Err(AppError::Forbidden(
            "Managers cannot approve their own monthly closing".to_string(),
        ));
    }
    authorize_scope(&state, &user, target).await?;
    let reason = normalized_reason(payload.reason.as_deref())?;
    let response = work_schedule::transition_monthly_closing(
        &state.write_pool,
        &user_id,
        payload.year,
        payload.month,
        MonthlyClosingStatus::Approved,
        &user.id.to_string(),
        reason,
    )
    .await
    .map_err(map_repository_error)?;
    Ok(Json(response))
}

pub async fn close_monthly_closing(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(user_id): Path<String>,
    Json(payload): Json<MonthlyClosingTransitionRequest>,
) -> Result<Json<MonthlyClosingWorkflowResponse>, AppError> {
    require_system_admin(&user)?;
    validate_monthly_transition_payload(&payload)?;
    UserId::from_str(&user_id).map_err(|_| invalid_work_schedule("invalid user_id"))?;
    let reason = normalized_reason(payload.reason.as_deref())?;
    // Transition to `closed` and lock the resolved workdays atomically so a
    // failure partway through cannot leave the workflow `closed` with
    // unlocked workdays (closed -> closed is not a valid retry transition).
    let response = work_schedule::close_monthly_closing_workflow(
        &state.write_pool,
        &user_id,
        payload.year,
        payload.month,
        &user.id.to_string(),
        reason,
    )
    .await
    .map_err(map_repository_error)?;
    Ok(Json(response))
}

pub async fn reopen_monthly_closing(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(user_id): Path<String>,
    Json(payload): Json<MonthlyClosingTransitionRequest>,
) -> Result<Json<MonthlyClosingWorkflowResponse>, AppError> {
    require_system_admin(&user)?;
    validate_monthly_transition_payload(&payload)?;
    UserId::from_str(&user_id).map_err(|_| invalid_work_schedule("invalid user_id"))?;
    let reason = normalized_reason(payload.reason.as_deref())?;
    let response = work_schedule::transition_monthly_closing(
        &state.write_pool,
        &user_id,
        payload.year,
        payload.month,
        MonthlyClosingStatus::Reopened,
        &user.id.to_string(),
        reason,
    )
    .await
    .map_err(map_repository_error)?;
    Ok(Json(response))
}

struct VersionDefinitionInput<'a> {
    effective_from: chrono::NaiveDate,
    effective_until: Option<chrono::NaiveDate>,
    timezone: &'a str,
    workday_boundary: chrono::NaiveTime,
    late_grace_minutes: i32,
    early_leave_grace_minutes: i32,
    schedule_type: WorkScheduleType,
    flex_policy: Option<&'a FlexPolicyInput>,
    days: &'a [timekeeper_contract::work_schedules::WeekdayRuleInput],
}

fn validate_version_definition(input: VersionDefinitionInput<'_>) -> Result<(), AppError> {
    let VersionDefinitionInput {
        effective_from,
        effective_until,
        timezone,
        workday_boundary,
        late_grace_minutes,
        early_leave_grace_minutes,
        schedule_type,
        flex_policy,
        days,
    } = input;
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
        schedule_type: domain_schedule_type(schedule_type),
        flex_policy: flex_policy.map(domain_flex_policy),
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

fn domain_schedule_type(value: WorkScheduleType) -> ScheduleType {
    match value {
        WorkScheduleType::Fixed => ScheduleType::Fixed,
        WorkScheduleType::Flex => ScheduleType::Flex,
    }
}

fn domain_flex_policy(policy: &FlexPolicyInput) -> FlexPolicy {
    FlexPolicy {
        settlement_period: SettlementPeriod {
            unit: match policy.settlement_period.unit {
                timekeeper_contract::work_schedules::SettlementPeriodUnit::Monthly => {
                    SettlementPeriodUnit::Monthly
                }
            },
            contracted_minutes_per_period: policy.settlement_period.contracted_minutes_per_period,
        },
        core_time_windows: policy
            .core_time_windows
            .iter()
            .map(|window| CoreTimeWindow {
                weekday: window.weekday,
                start_time: window.start_time,
                start_day_offset: window.start_day_offset,
                end_time: window.end_time,
                end_day_offset: window.end_day_offset,
            })
            .collect(),
    }
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
        WorkScheduleRepositoryError::InvalidStateTransition => AppError::ConflictWithCode {
            message: "Invalid monthly closing state transition".to_string(),
            code: "INVALID_MONTHLY_CLOSING_TRANSITION".to_string(),
        },
        WorkScheduleRepositoryError::CorruptData(message) => {
            AppError::InternalServerError(anyhow::anyhow!(message))
        }
        WorkScheduleRepositoryError::Sqlx(error) => AppError::InternalServerError(error.into()),
    }
}

fn repository_error_code_message(error: &WorkScheduleRepositoryError) -> (&'static str, String) {
    match error {
        WorkScheduleRepositoryError::NotFound => (
            "WORK_SCHEDULE_NOT_FOUND",
            "Work schedule resource not found".into(),
        ),
        WorkScheduleRepositoryError::CodeConflict => (
            "WORK_SCHEDULE_CODE_CONFLICT",
            "Work schedule code already exists".into(),
        ),
        WorkScheduleRepositoryError::PeriodOverlap => (
            "EFFECTIVE_PERIOD_OVERLAP",
            "Effective period overlaps an existing record".into(),
        ),
        WorkScheduleRepositoryError::PublishedVersionImmutable => (
            "PUBLISHED_VERSION_IMMUTABLE",
            "Published work schedule version is immutable".into(),
        ),
        WorkScheduleRepositoryError::RevisionConflict => {
            ("REVISION_CONFLICT", "Draft revision does not match".into())
        }
        WorkScheduleRepositoryError::RetiredSchedule => (
            "WORK_SCHEDULE_RETIRED",
            "Retired work schedule cannot be changed or assigned".into(),
        ),
        WorkScheduleRepositoryError::InvalidReference => (
            "INVALID_WORK_SCHEDULE_REFERENCE",
            "Referenced user or department does not exist".into(),
        ),
        WorkScheduleRepositoryError::InvalidStateTransition => (
            "INVALID_MONTHLY_CLOSING_TRANSITION",
            "Invalid monthly closing state transition".into(),
        ),
        WorkScheduleRepositoryError::CorruptData(message) => {
            ("INVALID_WORK_SCHEDULE", message.clone())
        }
        WorkScheduleRepositoryError::Sqlx(error) => {
            tracing::error!(error = %error, "bulk work schedule assignment target failed");
            (
                "WORK_SCHEDULE_REPOSITORY_ERROR",
                "internal error while processing this target".into(),
            )
        }
    }
}

fn projection_error(
    user_id: &str,
    work_date: NaiveDate,
    error: ResolveWorkdayError,
) -> WorkScheduleProjectionError {
    match error {
        ResolveWorkdayError::WorkScheduleNotConfigured => WorkScheduleProjectionError {
            user_id: user_id.to_string(),
            work_date,
            code: "WORK_SCHEDULE_NOT_CONFIGURED".to_string(),
            message: "work schedule is not configured".to_string(),
        },
        ResolveWorkdayError::InvalidScheduleData(message) => {
            tracing::error!(error = %message, "work schedule projection found invalid schedule data");
            WorkScheduleProjectionError {
                user_id: user_id.to_string(),
                work_date,
                code: "WORK_SCHEDULE_PROJECTION_FAILED".to_string(),
                message: "internal error while generating this projection".to_string(),
            }
        }
        ResolveWorkdayError::Repository(message) => {
            tracing::error!(error = %message, "work schedule projection repository error");
            WorkScheduleProjectionError {
                user_id: user_id.to_string(),
                work_date,
                code: "WORK_SCHEDULE_PROJECTION_FAILED".to_string(),
                message: "internal error while generating this projection".to_string(),
            }
        }
    }
}

fn failure(
    target: AssignmentTarget,
    code: &str,
    message: String,
) -> BulkWorkScheduleAssignmentFailure {
    BulkWorkScheduleAssignmentFailure {
        target,
        code: code.to_string(),
        message,
    }
}

async fn authorize_scope(state: &AppState, actor: &User, target: UserId) -> Result<(), AppError> {
    if actor.is_system_admin() {
        return Ok(());
    }
    if actor.is_manager()
        && can_manager_approve(state.read_pool(), actor.id, target)
            .await
            .map_err(|error| AppError::InternalServerError(error.into()))?
    {
        return Ok(());
    }
    Err(AppError::Forbidden("Forbidden".to_string()))
}

fn validate_range(from: NaiveDate, to: NaiveDate) -> Result<(), AppError> {
    if from > to {
        return Err(invalid_work_schedule("from must be on or before to"));
    }
    if to.signed_duration_since(from).num_days() > 366 {
        return Err(invalid_work_schedule(
            "requested range exceeds the supported maximum",
        ));
    }
    Ok(())
}

fn validate_overtime_monitor_settings(
    payload: &OvertimeMonitorSettingsRequest,
) -> Result<(), AppError> {
    if !(1..=12).contains(&payload.fiscal_year_start_month) {
        return Err(invalid_work_schedule(
            "fiscal_year_start_month must be between 1 and 12",
        ));
    }
    if payload.monthly_limit_minutes <= 0
        || payload.yearly_limit_minutes <= 0
        || payload.rolling_average_limit_minutes <= 0
        || payload.single_month_absolute_limit_minutes <= 0
    {
        return Err(invalid_work_schedule("overtime limits must be positive"));
    }
    if !(1..=100).contains(&payload.warning_ratio_percent) {
        return Err(invalid_work_schedule(
            "warning_ratio_percent must be between 1 and 100",
        ));
    }
    if payload.overtime_request_tolerance_minutes < 0 {
        return Err(invalid_work_schedule(
            "overtime_request_tolerance_minutes must be non-negative",
        ));
    }
    Ok(())
}

fn validate_monthly_transition_payload(
    payload: &MonthlyClosingTransitionRequest,
) -> Result<(), AppError> {
    if !(1..=12).contains(&payload.month) {
        return Err(invalid_work_schedule("month must be between 1 and 12"));
    }
    if !(1900..=9999).contains(&payload.year) {
        return Err(invalid_work_schedule("year must be between 1900 and 9999"));
    }
    normalized_reason(payload.reason.as_deref())?;
    Ok(())
}

fn normalized_reason(value: Option<&str>) -> Result<Option<&str>, AppError> {
    let reason = value.map(str::trim).filter(|v| !v.is_empty());
    if reason.is_some_and(|item| item.chars().count() > 500) {
        return Err(invalid_work_schedule(
            "reason must be at most 500 characters",
        ));
    }
    Ok(reason)
}

fn dates_inclusive(from: NaiveDate, to: NaiveDate) -> Result<Vec<NaiveDate>, AppError> {
    validate_range(from, to)?;
    let mut dates = Vec::new();
    let mut current = from;
    while current <= to {
        dates.push(current);
        current = current
            .succ_opt()
            .ok_or_else(|| AppError::InternalServerError(anyhow::anyhow!("date overflow")))?;
    }
    Ok(dates)
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

    #[test]
    fn repository_sql_errors_are_not_returned_to_bulk_clients() {
        let error = WorkScheduleRepositoryError::Sqlx(sqlx::Error::Protocol(
            "relation work_schedule_assignments leaked".to_string(),
        ));

        let (code, message) = repository_error_code_message(&error);

        assert_eq!(code, "WORK_SCHEDULE_REPOSITORY_ERROR");
        assert_eq!(message, "internal error while processing this target");
    }

    #[test]
    fn projection_repository_errors_are_not_returned_to_clients() {
        let error = ResolveWorkdayError::Repository("database table name leaked".to_string());

        let projection_error = projection_error("user-1", chrono::NaiveDate::MIN, error);

        assert_eq!(projection_error.code, "WORK_SCHEDULE_PROJECTION_FAILED");
        assert_eq!(
            projection_error.message,
            "internal error while generating this projection"
        );
    }
}
