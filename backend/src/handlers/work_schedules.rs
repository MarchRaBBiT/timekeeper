use axum::{
    extract::{Extension, Query, State},
    Json,
};
use chrono::Utc;
use timekeeper_app::user_workdays::{
    ListUserWorkdays, ListUserWorkdaysCommand, ListUserWorkdaysError,
};
use timekeeper_app::work_schedules::{
    ResolveWorkday, ResolveWorkdayCommand, ResolveWorkdayError, ResolvedDayKind, ResolvedWorkday,
    WorkScheduleSource,
};
use timekeeper_contract::work_schedules::{
    ResolvedBreakResponse, ResolvedDayKind as ContractResolvedDayKind,
    ResolvedWorkIntervalResponse, ResolvedWorkdayListResponse, ResolvedWorkdayRangeQuery,
    ResolvedWorkdayResponse, WorkScheduleSource as ContractWorkScheduleSource,
};
use timekeeper_domain::work_schedules::{PlannedBreak, PlannedWorkInterval};
use timekeeper_infra_postgres::work_schedules::WorkdayResolverPostgresRepository;

use crate::{error::AppError, models::user::User, state::AppState};

pub async fn get_my_workdays(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(query): Query<ResolvedWorkdayRangeQuery>,
) -> Result<Json<ResolvedWorkdayListResponse>, AppError> {
    let user_id = user.id.to_string();
    list_resolved_workdays(&state, &user_id, query).await
}

/// `from`/`to` の範囲で解決済み勤務日を取得し、応答 DTO へ整形する共通処理。
pub(crate) async fn list_resolved_workdays(
    state: &AppState,
    user_id: &str,
    query: ResolvedWorkdayRangeQuery,
) -> Result<Json<ResolvedWorkdayListResponse>, AppError> {
    materialize_resolved_workdays(state, user_id, query.from, query.to).await?;
    let repository = WorkdayResolverPostgresRepository::new(state.read_pool().clone());
    let use_case = ListUserWorkdays::new(repository);
    let workdays = use_case
        .execute(ListUserWorkdaysCommand {
            user_id: user_id.to_string(),
            from: query.from,
            to: query.to,
        })
        .await
        .map_err(list_workdays_error_to_app_error)?;
    Ok(Json(ResolvedWorkdayListResponse {
        from: query.from,
        to: query.to,
        items: workdays
            .into_iter()
            .map(resolved_workday_to_response)
            .collect(),
    }))
}

async fn materialize_resolved_workdays(
    state: &AppState,
    user_id: &str,
    from: chrono::NaiveDate,
    to: chrono::NaiveDate,
) -> Result<(), AppError> {
    let repository = WorkdayResolverPostgresRepository::new(state.write_pool.clone());
    let resolver = ResolveWorkday::new(repository.clone(), repository.clone(), repository);
    let resolved_at = Utc::now();
    let mut current = from;
    while current <= to {
        match resolver
            .execute(ResolveWorkdayCommand {
                user_id: user_id.to_string(),
                work_date: current,
                resolved_at,
            })
            .await
        {
            Ok(_) | Err(ResolveWorkdayError::WorkScheduleNotConfigured) => {}
            Err(error) => {
                return Err(AppError::InternalServerError(anyhow::anyhow!(
                    error.to_string()
                )));
            }
        }
        current = current
            .succ_opt()
            .ok_or_else(|| AppError::InternalServerError(anyhow::anyhow!("date overflow")))?;
    }
    Ok(())
}

fn list_workdays_error_to_app_error(error: ListUserWorkdaysError) -> AppError {
    match error {
        ListUserWorkdaysError::InvalidRange => AppError::BadRequestWithCode {
            message: "from must be on or before to".to_string(),
            code: "INVALID_WORK_SCHEDULE".to_string(),
        },
        ListUserWorkdaysError::RangeTooLarge => AppError::BadRequestWithCode {
            message: "requested range exceeds the supported maximum".to_string(),
            code: "INVALID_WORK_SCHEDULE".to_string(),
        },
        ListUserWorkdaysError::Repository(message) => {
            AppError::InternalServerError(anyhow::anyhow!(message))
        }
    }
}

pub(crate) fn resolved_workday_to_response(workday: ResolvedWorkday) -> ResolvedWorkdayResponse {
    ResolvedWorkdayResponse {
        id: workday.id,
        user_id: workday.user_id,
        work_date: workday.work_date,
        work_schedule_id: workday.work_schedule_id,
        work_schedule_version_id: workday.work_schedule_version_id,
        source: source_to_response(workday.source),
        day_kind: day_kind_to_response(workday.day_kind),
        timezone: workday.timezone,
        workday_boundary: workday.workday_boundary,
        expected_work_minutes: workday.expected_work_minutes,
        work_intervals: workday
            .work_intervals
            .into_iter()
            .map(interval_to_response)
            .collect(),
        planned_breaks: workday
            .planned_breaks
            .into_iter()
            .map(break_to_response)
            .collect(),
        resolved_at: workday.resolved_at,
        locked_at: workday.locked_at,
    }
}

fn interval_to_response(interval: PlannedWorkInterval) -> ResolvedWorkIntervalResponse {
    ResolvedWorkIntervalResponse {
        start_time: interval.start_time,
        start_day_offset: interval.start_day_offset,
        end_time: interval.end_time,
        end_day_offset: interval.end_day_offset,
    }
}

fn break_to_response(planned_break: PlannedBreak) -> ResolvedBreakResponse {
    ResolvedBreakResponse {
        start_time: planned_break.start_time,
        start_day_offset: planned_break.start_day_offset,
        end_time: planned_break.end_time,
        end_day_offset: planned_break.end_day_offset,
    }
}

fn source_to_response(source: WorkScheduleSource) -> ContractWorkScheduleSource {
    match source {
        WorkScheduleSource::Override => ContractWorkScheduleSource::Override,
        WorkScheduleSource::User => ContractWorkScheduleSource::User,
        WorkScheduleSource::Department => ContractWorkScheduleSource::Department,
        WorkScheduleSource::Organization => ContractWorkScheduleSource::Organization,
    }
}

fn day_kind_to_response(day_kind: ResolvedDayKind) -> ContractResolvedDayKind {
    match day_kind {
        ResolvedDayKind::ScheduledWorkday => ContractResolvedDayKind::ScheduledWorkday,
        ResolvedDayKind::ScheduledNonWorkingDay => ContractResolvedDayKind::ScheduledNonWorkingDay,
        ResolvedDayKind::PublicHoliday => ContractResolvedDayKind::PublicHoliday,
    }
}
