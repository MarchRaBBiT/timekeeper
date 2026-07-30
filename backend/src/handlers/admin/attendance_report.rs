use std::collections::HashMap;

use async_trait::async_trait;
use axum::{
    extract::{Extension, Query, State},
    Json,
};
use chrono::{Datelike, Duration, NaiveDate};
use sqlx::{FromRow, PgPool};
use timekeeper_app::{
    admin_attendance_report::{
        AdminAttendanceReportCommand, AdminAttendanceReportError, AdminAttendanceReportRepository,
        GetAdminAttendanceReport,
    },
    attendance_classification::{
        GetMonthlyClassification, MonthlyClassification, MonthlyClassificationQuery,
    },
};
use timekeeper_contract::{
    admin_attendance_report::{
        AdminAttendanceReportItem, AdminAttendanceReportQuery, AdminAttendanceReportResponse,
        ReportClassification,
    },
    work_schedules::{OvertimeMonitorStatus, WorkScheduleAnomalyKind},
};
use timekeeper_infra_postgres::attendance_classification::{
    BatchClassificationSnapshot, NoopClassificationMaterializer,
};

use crate::{
    error::AppError,
    models::user::User,
    repositories::{department::list_subordinate_user_ids, work_schedule},
    state::AppState,
    utils::time,
};

#[derive(Debug, Clone, FromRow)]
struct ScopedUserRow {
    id: String,
    username: String,
    department_id: Option<String>,
    department_name: Option<String>,
}

#[derive(Debug, Clone)]
struct PostgresAdminAttendanceReportRepository {
    pool: PgPool,
    today: NaiveDate,
}

#[async_trait]
impl AdminAttendanceReportRepository for PostgresAdminAttendanceReportRepository {
    async fn list_scoped_month(
        &self,
        actor_user_id: &str,
        actor_is_system_admin: bool,
        year: i32,
        month: u32,
        department_id: Option<&str>,
    ) -> Result<Vec<AdminAttendanceReportItem>, AdminAttendanceReportError> {
        let scoped_ids = if actor_is_system_admin {
            None
        } else {
            let actor_id = actor_user_id.parse().map_err(repository_error)?;
            if let Some(department_id) = department_id {
                ensure_department_scope(&self.pool, actor_user_id, department_id).await?;
            }
            Some(
                list_subordinate_user_ids(&self.pool, actor_id)
                    .await
                    .map_err(repository_error)?,
            )
        };
        let users = load_scoped_users(&self.pool, scoped_ids.as_deref(), department_id).await?;
        if users.is_empty() {
            return Ok(Vec::new());
        }
        let user_ids = users.iter().map(|user| user.id.clone()).collect::<Vec<_>>();
        let month_start = NaiveDate::from_ymd_opt(year, month, 1)
            .ok_or(AdminAttendanceReportError::InvalidMonth)?;
        let month_end = end_of_month(month_start)?;
        let classification_window_from = month_start - Duration::days(6);
        let classification_window_to = month_end + Duration::days(7);
        let classification_repository = BatchClassificationSnapshot::load(
            &self.pool,
            &user_ids,
            classification_window_from,
            classification_window_to,
        )
        .await
        .map_err(repository_error)?;
        let classification_use_case = GetMonthlyClassification::new(
            classification_repository,
            NoopClassificationMaterializer,
        );
        let mut classifications = HashMap::with_capacity(users.len());
        // T-03 owns projection and classification semantics. Run it before the
        // T-08/T-09 batch reads so a cold first report cannot observe stale
        // anomaly/overtime data from before projection materialization.
        for user in &users {
            let classification = classification_use_case
                .execute(MonthlyClassificationQuery {
                    user_id: user.id.clone(),
                    year,
                    month,
                })
                .await
                .map_err(repository_error)?;
            let classification = match classification {
                MonthlyClassification::Calculated(value)
                    if value.days.len()
                        < usize::try_from(month_end.day()).unwrap_or(usize::MAX) =>
                {
                    MonthlyClassification::UnresolvedDays
                }
                other => other,
            };
            classifications.insert(user.id.clone(), classification);
        }
        let anomalies = work_schedule::list_anomalies(
            &self.pool,
            Some(user_ids.clone()),
            month_start,
            month_end,
            self.today,
        )
        .await
        .map_err(repository_error)?;
        let overtime = work_schedule::list_overtime_monitor(&self.pool, &user_ids, year, month)
            .await
            .map_err(repository_error)?;

        let mut anomaly_counts: HashMap<String, (i64, i64, i64, i64)> = HashMap::new();
        for anomaly in anomalies {
            let counts = anomaly_counts.entry(anomaly.user_id).or_default();
            counts.3 += 1;
            match anomaly.kind {
                WorkScheduleAnomalyKind::Late => counts.0 += 1,
                WorkScheduleAnomalyKind::EarlyLeave => counts.1 += 1,
                WorkScheduleAnomalyKind::Absent => counts.2 += 1,
                _ => {}
            }
        }
        let statuses = overtime
            .items
            .into_iter()
            .map(|item| {
                let status = [
                    item.monthly_status,
                    item.yearly_status,
                    item.rolling_average_status,
                ]
                .into_iter()
                .max_by_key(|status| severity(*status))
                .unwrap_or(OvertimeMonitorStatus::Ok);
                (item.user_id, status)
            })
            .collect::<HashMap<_, _>>();
        let mut items = Vec::with_capacity(users.len());
        for user in users {
            let classification = classifications
                .remove(&user.id)
                .ok_or_else(|| repository_error("classification missing for scoped user"))?;
            let counts = anomaly_counts.remove(&user.id).unwrap_or_default();
            items.push(AdminAttendanceReportItem {
                overtime_status: statuses
                    .get(&user.id)
                    .copied()
                    .unwrap_or(OvertimeMonitorStatus::Ok),
                user_id: user.id,
                user_name: user.username,
                department_id: user.department_id,
                department_name: user.department_name,
                classification: classification_to_contract(classification),
                late_count: counts.0,
                early_leave_count: counts.1,
                absent_count: counts.2,
                anomaly_count: counts.3,
            });
        }
        Ok(items)
    }
}

pub async fn get_admin_attendance_report(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(query): Query<AdminAttendanceReportQuery>,
) -> Result<Json<AdminAttendanceReportResponse>, AppError> {
    if !user.is_system_admin() && !user.is_manager() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }
    if let Some(department_id) = query.department_id.as_deref() {
        department_id
            .parse::<crate::types::DepartmentId>()
            .map_err(|_| AppError::BadRequest("invalid department_id".into()))?;
    }
    let use_case = GetAdminAttendanceReport::new(PostgresAdminAttendanceReportRepository {
        pool: state.read_pool().clone(),
        today: time::today_local(&state.config.time_zone),
    });
    use_case
        .execute(AdminAttendanceReportCommand {
            actor_user_id: user.id.to_string(),
            actor_is_system_admin: user.is_system_admin(),
            query,
        })
        .await
        .map(Json)
        .map_err(report_error_to_app_error)
}

async fn ensure_department_scope(
    pool: &PgPool,
    actor_user_id: &str,
    department_id: &str,
) -> Result<(), AdminAttendanceReportError> {
    let allowed = sqlx::query_scalar::<_, bool>(
        "WITH RECURSIVE scoped_departments AS (
             SELECT department_id FROM department_managers WHERE user_id = $1
             UNION
             SELECT d.id FROM departments d
             JOIN scoped_departments parent ON d.parent_id = parent.department_id
         )
         SELECT EXISTS (
             SELECT 1 FROM scoped_departments WHERE department_id = $2
         )",
    )
    .bind(actor_user_id)
    .bind(department_id)
    .fetch_one(pool)
    .await
    .map_err(repository_error)?;
    if allowed {
        Ok(())
    } else {
        Err(AdminAttendanceReportError::ForbiddenDepartment)
    }
}

async fn load_scoped_users(
    pool: &PgPool,
    scoped_ids: Option<&[String]>,
    department_id: Option<&str>,
) -> Result<Vec<ScopedUserRow>, AdminAttendanceReportError> {
    sqlx::query_as::<_, ScopedUserRow>(
        "SELECT u.id, u.username, u.department_id, d.name AS department_name
         FROM users u LEFT JOIN departments d ON d.id = u.department_id
         WHERE ($1::TEXT[] IS NULL OR u.id = ANY($1))
           AND ($2::TEXT IS NULL OR u.department_id = $2)
         ORDER BY u.id",
    )
    .bind(scoped_ids)
    .bind(department_id)
    .fetch_all(pool)
    .await
    .map_err(repository_error)
}

fn classification_to_contract(value: MonthlyClassification) -> ReportClassification {
    match value {
        MonthlyClassification::Calculated(value) => ReportClassification::Calculated {
            actual_minutes: value.totals.actual_minutes,
            scheduled_minutes: value.totals.scheduled_minutes,
            statutory_within_minutes: value.totals.statutory_within_minutes,
            statutory_excess_minutes: value.totals.statutory_excess_minutes,
            legal_holiday_minutes: value.totals.legal_holiday_minutes,
            night_minutes: value.totals.night_minutes,
        },
        MonthlyClassification::UnresolvedDays => ReportClassification::UnresolvedDays,
        MonthlyClassification::WorkRuleNotConfigured => ReportClassification::WorkRuleNotConfigured,
    }
}

fn end_of_month(start: NaiveDate) -> Result<NaiveDate, AdminAttendanceReportError> {
    let (year, month) = if start.month() == 12 {
        (start.year() + 1, 1)
    } else {
        (start.year(), start.month() + 1)
    };
    NaiveDate::from_ymd_opt(year, month, 1)
        .map(|next| next - Duration::days(1))
        .ok_or(AdminAttendanceReportError::InvalidYear)
}

const fn severity(status: OvertimeMonitorStatus) -> u8 {
    match status {
        OvertimeMonitorStatus::Ok => 0,
        OvertimeMonitorStatus::Warning => 1,
        OvertimeMonitorStatus::Exceeded => 2,
    }
}

fn repository_error(error: impl std::fmt::Display) -> AdminAttendanceReportError {
    AdminAttendanceReportError::Repository(error.to_string())
}

fn report_error_to_app_error(error: AdminAttendanceReportError) -> AppError {
    match error {
        AdminAttendanceReportError::ForbiddenDepartment => {
            AppError::Forbidden("department is outside the actor scope".into())
        }
        AdminAttendanceReportError::InvalidYear
        | AdminAttendanceReportError::InvalidMonth
        | AdminAttendanceReportError::InvalidPage
        | AdminAttendanceReportError::InvalidPerPage => AppError::BadRequest(error.to_string()),
        AdminAttendanceReportError::Repository(message) => {
            AppError::InternalServerError(anyhow::anyhow!(message))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use timekeeper_app::attendance_classification::{
        CalculatedClassification, ClassificationTotals, FlexPeriodStatus,
    };

    #[test]
    fn calculated_classification_reuses_existing_totals_without_recalculation() {
        let classification = classification_to_contract(MonthlyClassification::Calculated(
            CalculatedClassification {
                year: 2026,
                month: 7,
                days: Vec::new(),
                totals: ClassificationTotals {
                    actual_minutes: 10,
                    scheduled_minutes: 20,
                    statutory_within_minutes: 30,
                    statutory_excess_minutes: 40,
                    legal_holiday_minutes: 50,
                    night_minutes: 60,
                },
                flex_period: FlexPeriodStatus::NotApplicable,
            },
        ));
        assert_eq!(
            classification,
            ReportClassification::Calculated {
                actual_minutes: 10,
                scheduled_minutes: 20,
                statutory_within_minutes: 30,
                statutory_excess_minutes: 40,
                legal_holiday_minutes: 50,
                night_minutes: 60,
            }
        );
    }

    #[test]
    fn report_keeps_both_unavailable_classification_states() {
        assert_eq!(
            classification_to_contract(MonthlyClassification::UnresolvedDays),
            ReportClassification::UnresolvedDays
        );
        assert_eq!(
            classification_to_contract(MonthlyClassification::WorkRuleNotConfigured),
            ReportClassification::WorkRuleNotConfigured
        );
    }

    #[test]
    fn month_end_handles_leap_year_and_december() {
        assert_eq!(
            end_of_month(NaiveDate::from_ymd_opt(2028, 2, 1).expect("date")),
            Ok(NaiveDate::from_ymd_opt(2028, 2, 29).expect("date"))
        );
        assert_eq!(
            end_of_month(NaiveDate::from_ymd_opt(2026, 12, 1).expect("date")),
            Ok(NaiveDate::from_ymd_opt(2026, 12, 31).expect("date"))
        );
    }
}
