use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde::Deserialize;
use serde_json::Value;
use sqlx::{FromRow, PgPool};
use timekeeper_app::{
    attendance_classification::{
        ActualBreak, ClassificationReadRepository, ClassificationWorkdayMaterializer, DayActuals,
        MonthlyClassificationError, WorkRuleRow,
    },
    work_schedules::{ResolveWorkday, ResolveWorkdayCommand, ResolveWorkdayError},
};
use timekeeper_domain::attendance_classification::WorkRuleParameters;
use uuid::Uuid;

use crate::work_schedules::{load_resolved_in_range, WorkdayResolverPostgresRepository};

#[derive(Debug, Clone)]
pub struct ClassificationPostgresRepository {
    pool: PgPool,
}

impl ClassificationPostgresRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(Debug, Clone)]
pub struct PostgresWorkdayMaterializer {
    pool: PgPool,
}

impl PostgresWorkdayMaterializer {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(Debug, Clone, FromRow)]
struct AttendanceRow {
    id: String,
    date: NaiveDate,
    clock_in_time: Option<NaiveDateTime>,
    clock_out_time: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, FromRow)]
struct BreakRow {
    attendance_id: String,
    break_start_time: NaiveDateTime,
    break_end_time: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, FromRow)]
struct EffectiveCorrectionRow {
    attendance_id: String,
    clock_in_time_corrected: Option<NaiveDateTime>,
    clock_out_time_corrected: Option<NaiveDateTime>,
    break_records_corrected_json: Value,
}

#[derive(Debug, Clone, FromRow)]
struct WorkRuleSettingsRow {
    valid_from: NaiveDate,
    statutory_daily_minutes: i32,
    statutory_weekly_minutes: i32,
    night_start: NaiveTime,
    night_end: NaiveTime,
    week_start_weekday: i16,
    legal_holiday_weekday: i16,
}

#[derive(Debug, Clone, Deserialize)]
struct CorrectionBreakJson {
    break_start_time: NaiveDateTime,
    break_end_time: Option<NaiveDateTime>,
}

#[async_trait]
impl ClassificationReadRepository for ClassificationPostgresRepository {
    async fn list_resolved_in_range(
        &self,
        user_id: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<timekeeper_app::work_schedules::ResolvedWorkday>, MonthlyClassificationError>
    {
        validate_uuid(user_id, "user_id")?;
        load_resolved_in_range(&self.pool, user_id, from, to)
            .await
            .map_err(resolve_error_to_classification_error)
    }

    async fn list_day_actuals_in_range(
        &self,
        user_id: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<DayActuals>, MonthlyClassificationError> {
        validate_uuid(user_id, "user_id")?;
        list_day_actuals(&self.pool, user_id, from, to).await
    }

    async fn find_settlement_minutes(
        &self,
        version_id: &str,
    ) -> Result<Option<i64>, MonthlyClassificationError> {
        let version_id = validate_uuid(version_id, "work_schedule_version_id")?;
        let value = sqlx::query_scalar::<_, i32>(
            "SELECT contracted_minutes_per_period
             FROM work_schedule_settlement_periods
             WHERE version_id = $1",
        )
        .bind(version_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(repository_error)?;
        Ok(value.map(i64::from))
    }

    async fn list_work_rules_effective_until(
        &self,
        until: NaiveDate,
    ) -> Result<Vec<WorkRuleRow>, MonthlyClassificationError> {
        let rows = sqlx::query_as::<_, WorkRuleSettingsRow>(
            "SELECT valid_from, statutory_daily_minutes, statutory_weekly_minutes,
                    night_start, night_end, week_start_weekday, legal_holiday_weekday
             FROM work_rule_settings
             WHERE valid_from <= $1
             ORDER BY valid_from ASC",
        )
        .bind(until)
        .fetch_all(&self.pool)
        .await
        .map_err(repository_error)?;
        rows.into_iter().map(work_rule_row_to_app).collect()
    }
}

#[async_trait]
impl ClassificationWorkdayMaterializer for PostgresWorkdayMaterializer {
    async fn materialize(
        &self,
        user_id: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<(), MonthlyClassificationError> {
        validate_uuid(user_id, "user_id")?;
        let repository = WorkdayResolverPostgresRepository::new(self.pool.clone());
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
                Err(error) => return Err(resolve_error_to_classification_error(error)),
            }
            current = current
                .succ_opt()
                .ok_or_else(|| repository_message("date overflow"))?;
        }
        Ok(())
    }
}

async fn list_day_actuals(
    pool: &PgPool,
    user_id: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<DayActuals>, MonthlyClassificationError> {
    let attendance = sqlx::query_as::<_, AttendanceRow>(
        "SELECT id, date, clock_in_time, clock_out_time
         FROM attendance
         WHERE user_id = $1 AND date BETWEEN $2 AND $3
         ORDER BY date ASC, clock_in_time ASC NULLS LAST",
    )
    .bind(user_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await
    .map_err(repository_error)?;
    if attendance.is_empty() {
        return Ok(Vec::new());
    }

    let attendance_ids: Vec<String> = attendance.iter().map(|row| row.id.clone()).collect();
    let corrections = effective_corrections_by_attendance_id(pool, &attendance_ids).await?;
    let raw_breaks = raw_breaks_by_attendance_id(pool, &attendance_ids).await?;

    let mut actuals = Vec::with_capacity(attendance.len());
    for row in attendance {
        if let Some(correction) = corrections.get(&row.id) {
            actuals.push(DayActuals {
                work_date: row.date,
                clock_in_time: correction.clock_in_time_corrected,
                clock_out_time: correction.clock_out_time_corrected,
                breaks: correction_breaks(&correction.break_records_corrected_json)?,
            });
        } else {
            actuals.push(DayActuals {
                work_date: row.date,
                clock_in_time: row.clock_in_time,
                clock_out_time: row.clock_out_time,
                breaks: raw_breaks.get(&row.id).cloned().unwrap_or_default(),
            });
        }
    }
    Ok(actuals)
}

async fn raw_breaks_by_attendance_id(
    pool: &PgPool,
    attendance_ids: &[String],
) -> Result<HashMap<String, Vec<ActualBreak>>, MonthlyClassificationError> {
    let rows = sqlx::query_as::<_, BreakRow>(
        "SELECT attendance_id, break_start_time, break_end_time
         FROM break_records
         WHERE attendance_id = ANY($1)
         ORDER BY attendance_id ASC, break_start_time ASC",
    )
    .bind(attendance_ids)
    .fetch_all(pool)
    .await
    .map_err(repository_error)?;
    let mut grouped: HashMap<String, Vec<ActualBreak>> = HashMap::new();
    for row in rows {
        grouped
            .entry(row.attendance_id)
            .or_default()
            .push(ActualBreak {
                break_start_time: row.break_start_time,
                break_end_time: row.break_end_time,
            });
    }
    Ok(grouped)
}

async fn effective_corrections_by_attendance_id(
    pool: &PgPool,
    attendance_ids: &[String],
) -> Result<HashMap<String, EffectiveCorrectionRow>, MonthlyClassificationError> {
    let rows = sqlx::query_as::<_, EffectiveCorrectionRow>(
        "SELECT attendance_id, clock_in_time_corrected, clock_out_time_corrected,
                break_records_corrected_json
         FROM attendance_correction_effective_values
         WHERE attendance_id = ANY($1)",
    )
    .bind(attendance_ids)
    .fetch_all(pool)
    .await
    .map_err(repository_error)?;
    let mut seen = HashSet::new();
    let mut by_id = HashMap::with_capacity(rows.len());
    for row in rows {
        if !seen.insert(row.attendance_id.clone()) {
            return Err(repository_message("duplicate effective correction row"));
        }
        by_id.insert(row.attendance_id.clone(), row);
    }
    Ok(by_id)
}

fn correction_breaks(value: &Value) -> Result<Vec<ActualBreak>, MonthlyClassificationError> {
    let breaks: Vec<CorrectionBreakJson> =
        serde_json::from_value(value.clone()).map_err(|error| {
            MonthlyClassificationError::Repository(format!(
                "invalid correction break_records_corrected_json: {error}"
            ))
        })?;
    Ok(breaks
        .into_iter()
        .map(|row| ActualBreak {
            break_start_time: row.break_start_time,
            break_end_time: row.break_end_time,
        })
        .collect())
}

fn work_rule_row_to_app(
    row: WorkRuleSettingsRow,
) -> Result<WorkRuleRow, MonthlyClassificationError> {
    Ok(WorkRuleRow {
        valid_from: row.valid_from,
        parameters: WorkRuleParameters {
            statutory_daily_minutes: i64::from(row.statutory_daily_minutes),
            statutory_weekly_minutes: i64::from(row.statutory_weekly_minutes),
            night_start: row.night_start,
            night_end: row.night_end,
            week_start_weekday: weekday(row.week_start_weekday, "week_start_weekday")?,
            legal_holiday_weekday: weekday(row.legal_holiday_weekday, "legal_holiday_weekday")?,
        },
    })
}

fn weekday(value: i16, field: &str) -> Result<u8, MonthlyClassificationError> {
    u8::try_from(value).map_err(|_| repository_message(&format!("invalid {field}")))
}

fn validate_uuid(value: &str, field: &str) -> Result<Uuid, MonthlyClassificationError> {
    Uuid::parse_str(value).map_err(|_| repository_message(&format!("invalid {field}")))
}

fn resolve_error_to_classification_error(error: ResolveWorkdayError) -> MonthlyClassificationError {
    MonthlyClassificationError::Repository(error.to_string())
}

fn repository_error(error: sqlx::Error) -> MonthlyClassificationError {
    MonthlyClassificationError::Repository(error.to_string())
}

fn repository_message(message: &str) -> MonthlyClassificationError {
    MonthlyClassificationError::Repository(message.to_string())
}
