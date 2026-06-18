use axum::{
    extract::{Extension, Query, State},
    http::{HeaderMap, HeaderValue},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;
use timekeeper_app::attendance::{
    ExportAdminAttendance as ExportAdminAttendanceUseCase,
    ExportAdminAttendanceError as AppExportAdminAttendanceError,
    ExportAdminAttendanceQuery as AppExportAdminAttendanceQuery,
};
use timekeeper_infra_postgres::attendance::AttendanceWorkflowRepository;
use utoipa::{IntoParams, ToSchema};

use crate::{
    error::AppError,
    models::user::User,
    state::AppState,
    utils::{
        csv::{render_admin_attendance_export_csv, AdminAttendanceExportCsvRow},
        encryption::decrypt_pii,
        pii::mask_name,
        time,
    },
};

use super::common::parse_date_value;

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct ExportQuery {
    pub username: Option<String>,
    pub from: Option<String>, // YYYY-MM-DD
    pub to: Option<String>,   // YYYY-MM-DD
}

pub async fn export_data(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(q): Query<ExportQuery>,
) -> Result<impl IntoResponse, AppError> {
    if !(user.is_manager() || user.is_system_admin()) {
        return Err(AppError::Forbidden("Forbidden".into()));
    }
    // Build filtered SQL
    let parsed_from = match q.from.as_deref() {
        Some(raw) => parse_date_value(raw)
            .ok_or(AppError::BadRequest("`from` must be a valid date".into()))
            .map(Some)?,
        None => None,
    };
    let parsed_to = match q.to.as_deref() {
        Some(raw) => parse_date_value(raw)
            .ok_or(AppError::BadRequest("`to` must be a valid date".into()))
            .map(Some)?,
        None => None,
    };
    if let (Some(from), Some(to)) = (parsed_from, parsed_to) {
        if from > to {
            return Err(AppError::BadRequest(
                "`from` must be on or before `to`".into(),
            ));
        }
    }

    let use_case = ExportAdminAttendanceUseCase::new(AttendanceWorkflowRepository::new(
        state.read_pool().clone(),
    ));
    let export = use_case
        .execute(AppExportAdminAttendanceQuery {
            requester_id: user.id.to_string(),
            requester_is_manager: user.is_manager(),
            requester_is_system_admin: user.is_system_admin(),
            username: q.username,
            from: parsed_from,
            to: parsed_to,
        })
        .await
        .map_err(export_admin_attendance_error_to_app_error)?;
    let csv_rows = export
        .rows
        .into_iter()
        .map(|row| {
            let full_name = decrypt_pii(&row.full_name_encrypted, &state.config)
                .unwrap_or_else(|_| "***".to_string());
            let full_name = if export.pii_masked {
                mask_name(&full_name)
            } else {
                full_name
            };
            AdminAttendanceExportCsvRow {
                username: row.username,
                full_name,
                date: row.date.format("%Y-%m-%d").to_string(),
                clock_in: row
                    .clock_in_time
                    .map(|time| time.format("%H:%M:%S").to_string())
                    .unwrap_or_default(),
                clock_out: row
                    .clock_out_time
                    .map(|time| time.format("%H:%M:%S").to_string())
                    .unwrap_or_default(),
                total_hours: row
                    .total_work_hours
                    .map(|hours| format!("{hours:.2}"))
                    .unwrap_or_else(|| "0.00".to_string()),
                status: row.status,
            }
        })
        .collect::<Vec<_>>();
    let csv_data = render_admin_attendance_export_csv(&csv_rows);

    let mut headers = HeaderMap::new();
    headers.insert(
        "X-PII-Masked",
        HeaderValue::from_static(if export.pii_masked { "true" } else { "false" }),
    );
    Ok((
        headers,
        Json(json!({
            "csv_data": csv_data,
            "filename": format!(
                "attendance_export_{}.csv",
                time::now_in_timezone(&state.config.time_zone).format("%Y%m%d_%H%M%S")
            )
        })),
    ))
}

fn export_admin_attendance_error_to_app_error(error: AppExportAdminAttendanceError) -> AppError {
    match error {
        AppExportAdminAttendanceError::Forbidden => AppError::Forbidden("Forbidden".into()),
        AppExportAdminAttendanceError::Repository(message) => {
            AppError::InternalServerError(anyhow::anyhow!(message))
        }
    }
}
