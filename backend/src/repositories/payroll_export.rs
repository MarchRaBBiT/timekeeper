use sqlx::{FromRow, PgPool};
use std::collections::HashMap;
use timekeeper_app::payroll_export::render_csv;
use timekeeper_contract::payroll_export::{
    PayrollExportFailedUser, PayrollExportResponse, PayrollExportedUser, PayrollSnapshot,
};

use super::work_schedule::{RepositoryResult, WorkScheduleRepositoryError};

#[derive(FromRow)]
struct SnapshotRow {
    user_id: String,
    year: i32,
    month: i32,
    revision: i32,
    worked_minutes: i64,
    scheduled_minutes: i64,
    statutory_within_minutes: i64,
    statutory_excess_minutes: i64,
    legal_holiday_minutes: i64,
    night_minutes: i64,
    absent_days: i32,
    paid_leave_days: i32,
    paid_leave_half_days: i32,
    paid_leave_minutes: i64,
    holiday_work_minutes: i64,
    substitute_holiday_days: i32,
    compensatory_leave_minutes: i64,
}

impl From<SnapshotRow> for PayrollSnapshot {
    fn from(row: SnapshotRow) -> Self {
        Self {
            employee_id: row.user_id,
            year: row.year,
            month: row.month,
            revision: row.revision,
            worked_minutes: row.worked_minutes,
            scheduled_minutes: row.scheduled_minutes,
            statutory_within_minutes: row.statutory_within_minutes,
            statutory_excess_minutes: row.statutory_excess_minutes,
            legal_holiday_minutes: row.legal_holiday_minutes,
            night_minutes: row.night_minutes,
            absent_days: row.absent_days,
            paid_leave_days: row.paid_leave_days,
            paid_leave_half_days: row.paid_leave_half_days,
            paid_leave_minutes: row.paid_leave_minutes,
            holiday_work_minutes: row.holiday_work_minutes,
            substitute_holiday_days: row.substitute_holiday_days,
            compensatory_leave_minutes: row.compensatory_leave_minutes,
        }
    }
}

pub async fn export_payroll(
    pool: &PgPool,
    year: i32,
    month: u32,
) -> RepositoryResult<PayrollExportResponse> {
    let month = i32::try_from(month)
        .map_err(|_| WorkScheduleRepositoryError::CorruptData("invalid export month".into()))?;
    // Read the current workflow state and its latest immutable snapshot in one
    // statement. PostgreSQL gives the statement one MVCC snapshot, so a
    // concurrent reopen cannot be observed separately from the snapshot read.
    let closed_snapshots = sqlx::query_as::<_, SnapshotRow>(
        "SELECT s.user_id, s.year, s.month, s.revision, s.worked_minutes,
                s.scheduled_minutes, s.statutory_within_minutes,
                s.statutory_excess_minutes, s.legal_holiday_minutes,
                s.night_minutes, s.absent_days, s.paid_leave_days,
                s.paid_leave_half_days, s.paid_leave_minutes,
                s.holiday_work_minutes, s.substitute_holiday_days,
                s.compensatory_leave_minutes
         FROM monthly_closing_workflows w
         JOIN LATERAL (
             SELECT ps.user_id, ps.year, ps.month, ps.revision, ps.worked_minutes,
                    ps.scheduled_minutes, ps.statutory_within_minutes,
                    ps.statutory_excess_minutes, ps.legal_holiday_minutes,
                    ps.night_minutes, ps.absent_days, ps.paid_leave_days,
                    ps.paid_leave_half_days, ps.paid_leave_minutes,
                    ps.holiday_work_minutes, ps.substitute_holiday_days,
                    ps.compensatory_leave_minutes
             FROM payroll_export_snapshots ps
             WHERE ps.user_id = w.user_id AND ps.year = w.year AND ps.month = w.month
             ORDER BY ps.revision DESC
             LIMIT 1
         ) s ON TRUE
         WHERE w.year = $1 AND w.month = $2 AND w.status = 'closed'",
    )
    .bind(year)
    .bind(month)
    .fetch_all(pool)
    .await?;
    let mut snapshots_by_user: HashMap<String, SnapshotRow> = closed_snapshots
        .into_iter()
        .map(|row| (row.user_id.clone(), row))
        .collect();
    let users = sqlx::query_scalar::<_, String>("SELECT id FROM users ORDER BY id")
        .fetch_all(pool)
        .await?;

    let mut exported = Vec::new();
    let mut failed = Vec::new();
    let mut snapshots = Vec::new();
    for user_id in users {
        match snapshots_by_user.remove(&user_id) {
            Some(row) => {
                exported.push(PayrollExportedUser {
                    user_id,
                    revision: row.revision,
                });
                snapshots.push(row.into());
            }
            None => failed.push(PayrollExportFailedUser {
                user_id,
                code: "monthly_not_closed".into(),
            }),
        }
    }
    Ok(PayrollExportResponse {
        year,
        month: u32::try_from(month)
            .map_err(|_| WorkScheduleRepositoryError::CorruptData("invalid export month".into()))?,
        exported,
        failed,
        csv: render_csv(&snapshots),
    })
}
