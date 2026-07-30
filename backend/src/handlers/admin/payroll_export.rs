use axum::{
    extract::{Extension, Query, State},
    Json,
};
use timekeeper_contract::payroll_export::{PayrollExportQuery, PayrollExportResponse};

use crate::{
    error::AppError,
    models::user::User,
    repositories::{payroll_export, work_schedule::WorkScheduleRepositoryError},
    state::AppState,
};

use super::work_schedules::require_system_admin;

pub async fn export_payroll(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(query): Query<PayrollExportQuery>,
) -> Result<Json<PayrollExportResponse>, AppError> {
    require_system_admin(&user)?;
    if !(1..=12).contains(&query.month) || !(2000..=2100).contains(&query.year) {
        return Err(AppError::Validation(vec![
            "year must be 2000..=2100 and month must be 1..=12".into(),
        ]));
    }
    // Payroll export must not use a potentially stale replica: the current
    // monthly-closing state gates whether an immutable snapshot is exportable.
    payroll_export::export_payroll(&state.write_pool, query.year, query.month)
        .await
        .map(Json)
        .map_err(map_repository_error)
}

fn map_repository_error(error: WorkScheduleRepositoryError) -> AppError {
    tracing::error!(error = %error, "payroll export failed");
    AppError::InternalServerError(anyhow::anyhow!("payroll export failed"))
}
